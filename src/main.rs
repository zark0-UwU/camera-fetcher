use std::{error::Error, thread, time::{ Duration, Instant}};

use reqwest::blocking::{self};


fn main() {
    let start = Instant::now();
    let study_interval = Duration::from_secs(5*60); // 5 minutes
    let total_elapsed = start.elapsed();
    let snapshot_interval = Duration::from_secs(60*2); // 2 minutes
    
    let mut last_downloaded = "";

    let url = "https://traficosevilla.es/camaras/cam28.jpg";
    loop {
        let loop_start = Instant::now();
        
        // ---- Task start ----
        println!("Running task at {:?}", loop_start);
        
        let _ = fetch_image(url, &last_downloaded).inspect(|new| last_downloaded = new);
        // ---- Task end ----

        let elapsed = loop_start.elapsed();

        if total_elapsed > study_interval{
            break;
        }
        if elapsed < snapshot_interval {
            thread::sleep(snapshot_interval - elapsed);
        } else {
            // If the task took longer than the snapshot_interval, continue immediately.
            eprintln!("Warning: task took longer ({:?}) than snapshot_interval {:?}", elapsed, snapshot_interval);
        }
    }
}


// fetches the image url, and checks the asociated date, if date!=last_downloaded image is saved to file system and returns ok
fn fetch_image<'a>(image_url:&str, last_downloaded:&str) -> Result<&'a str, Box<dyn Error>>{
    //request.send().inspect(|res| save_if_not_present(res,id));
    
    let resp = blocking::get(image_url)?;
    let date = "";
    if resp.status().is_success() {
        //let date = resp?.headers().get("date");//extract the header "date" as &str
/*         if let Some(value) = resp.headers().get("date") {
            date = value.to_str()?.clone(); // returns &str or Err if not valid UTF-8
            println!("date: {}", date);
        } else {
            println!("<date>Header not present");
        } */
        let date = resp
        .headers()
        .get("date")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_owned())
        .unwrap_or_default(); // take an owned String

        if  date != last_downloaded { // if the date of the image is not the same as the last one, save it.
            //last_downloaded = &date;
            let img_bytes = resp.bytes()?; // consume response
            //let img_bytes = res.bytes().unwrap_or_default();
            let image = image::load_from_memory(&img_bytes).unwrap_or_default();
            let _ = image.save_with_format(format!("./images/{:?}.webp", date) , image::ImageFormat::WebP);
        }
    } else {
        eprintln!("Request failed: {}", resp.status());
    }

    Ok(&date)
        
}

