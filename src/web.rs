use std::fs;
use std::io::{self, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Component, Path, PathBuf};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

/// Serves files from the `images` directory on port 8000.
pub fn serve(paused: Arc<AtomicBool>) -> io::Result<()> {
    let root = fs::canonicalize("images")?;
    let listener = TcpListener::bind("0.0.0.0:8000")?;
    for connection in listener.incoming() {
        if let Ok(stream) = connection {
            handle_connection(stream, &root, &paused)?;
        }
    }
    Ok(())
}

fn handle_connection(mut stream: TcpStream, root: &Path, paused: &AtomicBool) -> io::Result<()> {
    let mut request = [0; 8192];
    let count = stream.read(&mut request)?;
    let request = String::from_utf8_lossy(&request[..count]);
    let mut parts = request.lines().next().unwrap_or("").split_whitespace();
    let method = parts.next().unwrap_or("");
    let request_path = parts.next().unwrap_or("");
    let (path, query) = request_path.split_once('?').unwrap_or((request_path, ""));

    if path == "/save-config" {
        if method != "POST" {
            return respond(
                &mut stream,
                "405 Method Not Allowed",
                "text/plain",
                b"Use POST\n",
                method == "HEAD",
            );
        }
        return save_config(&mut stream, &request, false);
    }

    if path == "/run-state" {
        match method {
            "GET" | "HEAD" => {}
            "POST" => {
                let value = query.split('&').find_map(|part| {
                    let (key, value) = part.split_once('=')?;
                    (key == "paused").then_some(value)
                });

                match value {
                    Some("true") => paused.store(true, Ordering::Relaxed),
                    Some("false") => paused.store(false, Ordering::Relaxed),
                    _ => {
                        return respond(
                            &mut stream,
                            "400 Bad Request",
                            "text/plain",
                            b"Expected paused=true or paused=false\n",
                            false,
                        );
                    }
                }
            }
            _ => {
                return respond(
                    &mut stream,
                    "405 Method Not Allowed",
                    "text/plain",
                    b"Method not allowed\n",
                    false,
                );
            }
        }

        return respond_run_state(&mut stream, paused, method == "HEAD");
    }

    if method != "GET" && method != "HEAD" {
        return respond(
            &mut stream,
            "405 Method Not Allowed",
            "text/plain",
            b"Method not allowed\n",
            method == "HEAD",
        );
    }

    let path = path.split('?').next().unwrap_or("/");
    let decoded_path = match percent_decode(path) {
        Some(path) => path,
        None => {
            return respond(
                &mut stream,
                "400 Bad Request",
                "text/plain",
                b"Invalid URL path\n",
                method == "HEAD",
            );
        }
    };

    if path == "/" || path == "/index.html" || path == "/settings" {
        return respond(
            &mut stream,
            "200 OK",
            "text/html; charset=utf-8",
            UI,
            method == "HEAD",
        );
    }
    if path == "/config" {
        return serve_config(&mut stream, method == "HEAD");
    }
    if path == "/files" {
        return list_files(&mut stream, root, method == "HEAD");
    }
    if path == "/download" {
        let folder = query
            .split('&')
            .find_map(|part| {
                let (key, value) = part.split_once('=')?;
                (key == "folder").then_some(value)
            })
            .unwrap_or("");

        let Some(folder) = percent_decode(folder) else {
            return respond(
                &mut stream,
                "400 Bad Request",
                "text/plain",
                b"Invalid folder\n",
                method == "HEAD",
            );
        };

        return serve_folder_tar(&mut stream, root, &folder, method == "HEAD");
    }
    let mut relative = PathBuf::new();
    for component in Path::new(decoded_path.trim_start_matches('/')).components() {
        match component {
            Component::Normal(part) => relative.push(part),
            Component::CurDir => {}
            _ => {
                return respond(
                    &mut stream,
                    "403 Forbidden",
                    "text/plain",
                    b"Forbidden\n",
                    method == "HEAD",
                );
            }
        }
    }

    let mut target = root.join(relative);
    if target.is_dir() {
        target.push("index.html");
    }
    let canonical = match fs::canonicalize(&target) {
        Ok(path) if path.starts_with(root) => path,
        _ => {
            return respond(
                &mut stream,
                "404 Not Found",
                "text/plain",
                b"Not found\n",
                method == "HEAD",
            );
        }
    };
    let body = match fs::read(&canonical) {
        Ok(body) => body,
        Err(_) => {
            return respond(
                &mut stream,
                "404 Not Found",
                "text/plain",
                b"Not found\n",
                method == "HEAD",
            );
        }
    };
    let content_type = match canonical
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
        .as_str()
    {
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        "bmp" => "image/bmp",
        "txt" => "text/plain; charset=utf-8",
        "html" => "text/html; charset=utf-8",
        _ => "application/octet-stream",
    };
    respond(&mut stream, "200 OK", content_type, &body, method == "HEAD")
}

const UI: &[u8] = include_bytes!("web/index.html");

fn serve_config(stream: &mut TcpStream, head_only: bool) -> io::Result<()> {
    match fs::read("./config/camera-fetcher.yml") {
        Ok(body) => respond(
            stream,
            "200 OK",
            "text/yaml; charset=utf-8",
            &body,
            head_only,
        ),
        Err(_) => respond(
            stream,
            "404 Not Found",
            "text/plain",
            b"Configuration not found\n",
            head_only,
        ),
    }
}

fn save_config(stream: &mut TcpStream, request: &str, head_only: bool) -> io::Result<()> {
    let Some((headers, body)) = request.split_once("\r\n\r\n") else {
        return respond(
            stream,
            "400 Bad Request",
            "text/plain",
            b"Bad request\n",
            head_only,
        );
    };
    let length = headers
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("content-length")
                .then(|| value.trim().parse::<usize>().ok())
                .flatten()
        })
        .unwrap_or(0);
    if length > 1024 * 1024 || body.len() < length {
        return respond(
            stream,
            "400 Bad Request",
            "text/plain",
            b"Invalid configuration size\n",
            head_only,
        );
    }
    fs::write("./config/camera-fetcher.yml", &body.as_bytes()[..length])?;
    respond(stream, "200 OK", "text/plain", b"Saved\n", head_only)
}

fn list_files(stream: &mut TcpStream, root: &Path, head_only: bool) -> io::Result<()> {
    let mut names = Vec::new();
    fn walk(dir: &Path, root: &Path, names: &mut Vec<String>) -> io::Result<()> {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                walk(&path, root, names)?;
            } else if path.is_file() {
                let relative = path
                    .strip_prefix(root)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .replace('\\', "/");
                names.push(relative);
            }
        }
        Ok(())
    }
    if walk(root, root, &mut names).is_err() {
        return respond(
            stream,
            "500 Internal Server Error",
            "text/plain",
            b"Unable to list images\n",
            head_only,
        );
    }
    names.sort();
    let json = format!(
        "[{}]",
        names
            .iter()
            .map(|name| format!(
                "\"{}\"",
                name.replace('\\', "\\\\")
                    .replace('"', "\\\"")
                    .replace('\n', "\\n")
            ))
            .collect::<Vec<_>>()
            .join(",")
    );
    respond(
        stream,
        "200 OK",
        "application/json",
        json.as_bytes(),
        head_only,
    )
}

fn respond(
    stream: &mut TcpStream,
    status: &str,
    content_type: &str,
    body: &[u8],
    head_only: bool,
) -> io::Result<()> {
    write!(
        stream,
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    )?;
    if !head_only {
        stream.write_all(body)?;
    }
    Ok(())
}

fn percent_decode(path: &str) -> Option<String> {
    let bytes = path.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;

    while index < bytes.len() {
        if bytes[index] == b'%' {
            let hex = std::str::from_utf8(bytes.get(index + 1..index + 3)?).ok()?;
            decoded.push(u8::from_str_radix(hex, 16).ok()?);
            index += 3;
        } else {
            decoded.push(bytes[index]);
            index += 1;
        }
    }

    String::from_utf8(decoded).ok()
}

fn serve_folder_tar(
    stream: &mut TcpStream,
    root: &Path,
    folder: &str,
    head_only: bool,
) -> io::Result<()> {
    let mut target = root.to_path_buf();

    for component in Path::new(folder).components() {
        match component {
            Component::Normal(part) => target.push(part),
            Component::CurDir => {}
            _ => {
                return respond(
                    stream,
                    "400 Bad Request",
                    "text/plain",
                    b"Invalid folder\n",
                    head_only,
                );
            }
        }
    }

    let target = match fs::canonicalize(target) {
        Ok(path) if path.starts_with(root) && path.is_dir() => path,
        _ => {
            return respond(
                stream,
                "404 Not Found",
                "text/plain",
                b"Folder not found\n",
                head_only,
            );
        }
    };

    let mut archive = tar::Builder::new(Vec::new());
    archive.append_dir_all("images", &target)?;
    let body = archive.into_inner()?;

    write!(
        stream,
        "HTTP/1.1 200 OK\r\nContent-Type: application/x-tar\r\nContent-Disposition: attachment; filename=\"images.tar\"\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    )?;
    if !head_only {
        stream.write_all(&body)?;
    }
    Ok(())
}

fn respond_run_state(
    stream: &mut TcpStream,
    paused: &AtomicBool,
    head_only: bool,
) -> io::Result<()> {
    let state = if paused.load(Ordering::Relaxed) {
        b"paused\n".as_slice()
    } else {
        b"running\n".as_slice()
    };
    respond(stream, "200 OK", "text/plain", state, head_only)
}
