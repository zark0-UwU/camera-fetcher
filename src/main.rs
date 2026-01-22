use std::{
    error::Error,
    thread,
    time::{Duration, Instant},
};

use reqwest::blocking::{self};

fn main() {
    let start = Instant::now();
    let study_interval = Duration::from_secs(24 * 60 * 60); // 24h minutes
    let snapshot_interval = Duration::from_secs(60 * 2); // every 2 minutes
    let url = "https://traficosevilla.es/camaras/cam28.jpg"; // using this cammera

    let mut last_downloaded = "".to_string();

    loop {
        let loop_start = Instant::now();

        // ---- Task start ----
        println!("\nRunning task at {:?}", loop_start);

        let _ = fetch_image(url, &last_downloaded).inspect(|new| last_downloaded = new.clone());
        // ---- Task end ----

        let elapsed = loop_start.elapsed();
        let total_elapsed = start.elapsed();

        if total_elapsed > study_interval {
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
fn fetch_image<'a>(image_url: &str, last_downloaded: &str) -> Result<String, Box<dyn Error>> {
    print!("requesting data");
    let resp = blocking::get(image_url)?;
    let date: String;
    if resp.status().is_success() {
        date = resp
            .headers()
            .get("last-modified")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_owned())
            .unwrap_or_default(); // take an owned String

        print!(" => succesfull http request!\n");

        if date != last_downloaded {
            // if the date of the image is not the same as the last one, save it.
            let img_bytes = resp.bytes()?; // consume response
            let image = image::load_from_memory(&img_bytes).unwrap_or_default();
            let path = format!("./images/{:?}.webp", date.clone());
            //let path = "../images/test.webp";
            let _ = image.save_with_format(path.clone(), image::ImageFormat::WebP);
            println!("new image at {}", path);
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
