use chrono::{Datelike, NaiveDateTime};
use reqwest::blocking::{self};
use std::{error::Error, fs, path::Path};

#[derive(Clone)]
pub struct Camera {
    pub url: String,
    pub name: String,
    pub last_downloaded: String,
}

impl Camera {
    /*     fn new_from_name(camera_name: &str) -> Camera{
            Camera {
            url: format!("https://trafico.sevilla.org/camaras/{}.jpg", camera_name).to_string(),
            name: camera_name.to_string(),
            last_downloaded: String::from(""),
        }
    }
    */
    pub fn new_from_id(camera_id: i16, name: &str) -> Camera {
        Self::new(
            &format!("https://trafico.sevilla.org/camaras/cam{}.jpg", camera_id),
            name,
        )
    }
    pub fn new(url: &str, name: &str) -> Camera {
        Camera {
            url: url.to_string(),
            name: name.to_string(),
            last_downloaded: String::from(""),
        }
    }
    // fetches the image url, and checks the asociated date, if date!=last_downloaded image is saved to file system and returns ok
    pub fn fetch_image(&self, save_path: &str) -> Result<String, Box<dyn Error>> {
        print!("requesting data");
        let resp = blocking::get(self.url.clone())?;
        let date: String;
        if resp.status().is_success() {
            date = resp
                .headers()
                .get("last-modified")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_owned())
                .unwrap_or_default(); // take an owned String

            print!(" => succesfull http request!\n");

            if date != self.last_downloaded {
                // if the date of the image is not the same as the last one, save it.
                let img_bytes = resp.bytes()?; // consume response
                let image = image::load_from_memory(&img_bytes).unwrap_or_default();

                // example: Thu, 05 Feb 2026 16:11:22 GMT
                let parsed = NaiveDateTime::parse_from_str(&date, "%a, %d %b %Y %H:%M:%S GMT");
                if parsed.is_err() {
                    println!("error parsing date")
                } else {
                    let parsed_utc = parsed.unwrap().and_utc();
                    let month = parsed_utc.month();
                    let day = parsed_utc.day();

                    // make file name compliant with file system restrictions
                    let new_filename = format!(
                        "{}.webp",
                        parsed_utc
                            .to_rfc3339_opts(chrono::SecondsFormat::Secs, false)
                            .replace(":", ".")
                    );

                    // construct the file path
                    let str_path = format!("{}/{}/{}/{}", save_path, self.name, month, day);
                    let folder_path = Path::new(&str_path);

                    //if folder does not exist create it
                    if folder_path.try_exists().is_ok_and(|v| !v) {
                        let _ = fs::create_dir_all(folder_path);
                    }

                    let file_path = folder_path.join(new_filename);

                    // Save image file
                    let _ = image.save_with_format(file_path.clone(), image::ImageFormat::WebP);
                    println!("new image at {}", file_path.to_string_lossy());
                }
                Ok(date.clone())
            } else {
                Ok(date.clone())
            }
        } else {
            let msg = format!("Request failed: {}", resp.status());
            eprintln!("{}", msg);
            Err(msg.into())
        }
    }
}
