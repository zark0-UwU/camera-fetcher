use std::{
    error::Error, fs, path::Path, thread, time::{Duration, Instant}
};
use chrono::{NaiveDateTime, Datelike};

use reqwest::blocking::{self};

#[derive(Clone)]
struct Camera {
    url: String,
    name: String,
    last_downloaded: String,
}

impl Camera {
/*     fn new_from_name(camera_name: &str) -> Camera{
            Camera {
            url: format!("https://trafico.sevilla.org/camaras/{}.jpg", camera_name).to_string(),
            name: camera_name.to_string(),
            last_downloaded: String::from(""),
        }
    }
    fn new_from_id(camera_id: i16) -> Camera{
            Camera {
            url: format!("https://trafico.sevilla.org/camaras/cam{}.jpg", camera_id).to_string(),
            name: format!("cam{}",camera_id).to_string(),
            last_downloaded: String::from(""),
        }
    } */
    fn new(url: &str, name: &str) -> Camera{
            Camera {
            url: url.to_string(),
            name: name.to_string(),
            last_downloaded: String::from(""),
        }
    }
}

fn main() {
    
    let mut cameras = [
        Camera::new("https://trafico.sevilla.org/camaras/cam28.jpg", "Plaza de cuba (puente de san telmo)"),
        Camera::new("https://trafico.sevilla.org/camaras/cam9.jpg", "Puente Isabel II - Paseo Colon - Arjona"),
        Camera::new("https://trafico.sevilla.org/camaras/cam73.jpg", "Puerta de la carne - Menendez y Pelayo"),
        Camera::new("https://trafico.sevilla.org/camaras/cam26.jpg", "Av de la Palmera - Bueno Monreal"),
        Camera::new("https://trafico.sevilla.org/camaras/cam88.jpg", "Kansas city - Luis Montoto"),
        Camera::new("https://trafico.sevilla.org/camaras/cam23.jpg", "Glorieta de Santa Justa"),
        Camera::new("https://trafico.sevilla.org/camaras/cam10.jpg", "Ctra de carmona - Av de Llanes"),
    ];

    let start = Instant::now();
    let study_interval = Duration::from_secs(24 * 60 * 60); // 24h minutes
    let endless_study = true; // ignores study interval making the script run continusly
    let snapshot_interval = Duration::from_secs(60 * 2); // every 2 minutes

    println!("Endless mode: {}", endless_study);

    loop {
        let loop_start = Instant::now();

        // ---- Task start ----
        let mut cam_index = 0;
        while cam_index < cameras.len(){
            println!("\nRunning task at {:?}", loop_start);
            let _ = fetch_image(cameras[cam_index].clone()).inspect(|new| cameras[cam_index].last_downloaded = new.clone());
            cam_index += 1;
        }
        // ---- Task end ----

        let elapsed = loop_start.elapsed();
        let total_elapsed = start.elapsed();

        if !endless_study && total_elapsed > study_interval {
            println!("\nENDED STUDY INTERVAL");
            break;
        }
        if elapsed < snapshot_interval {
            thread::sleep(snapshot_interval - elapsed);
        } else {
            // If the task took longer than the snapshot_interval, continue immediately.
            eprintln!(
                "Warning: task took longer ({:?}) than snapshot_interval {:?}",
                elapsed, snapshot_interval
            );
        }
    }
}

// fetches the image url, and checks the asociated date, if date!=last_downloaded image is saved to file system and returns ok
fn fetch_image<'a>(camera: Camera) -> Result<String, Box<dyn Error>> {
    print!("requesting data");
    let resp = blocking::get(camera.url)?;
    let date: String;
    if resp.status().is_success() {
        date = resp
            .headers()
            .get("last-modified")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_owned())
            .unwrap_or_default(); // take an owned String

        print!(" => succesfull http request!\n");

        if date != camera.last_downloaded {
            // if the date of the image is not the same as the last one, save it.
            let img_bytes = resp.bytes()?; // consume response
            let image = image::load_from_memory(&img_bytes).unwrap_or_default();
             
            // example: Thu, 05 Feb 2026 16:11:22 GMT
            let parsed = NaiveDateTime::parse_from_str(&date, "%a, %d %b %Y %H:%M:%S GMT");
            if parsed.is_err(){
                println!("error parsing date")
            }else{
                let parsed_utc = parsed.unwrap().and_utc();
                let month = parsed_utc.month();
                let day = parsed_utc.day();

                // make file name compliant with file system restrictions
                let new_filename = format!("{}.webp",parsed_utc.to_rfc3339_opts(chrono::SecondsFormat::Secs, false).replace(":", "."));
                
                // construct the file path
                let str_path = format!("./images/{}/{}/{}", camera.name, month, day);
                let folder_path =  Path::new(&str_path);
                
                //if folder does not exist create it
                if folder_path.try_exists().is_ok_and(|v| !v){
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
