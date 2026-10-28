mod camera;
mod web;
use crate::camera::Camera;
use serde::Deserialize;
use std::{
    error::Error,
    fs,
    path::{Path, PathBuf},
    print,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

#[derive(Debug, Deserialize)]
struct Config {
    save_path: PathBuf,
    fetching_paused_at_startup: bool,
    username: String,
    password: String,
}

#[derive(Debug, Deserialize)]
struct CamerasCfg {
    cameras: Vec<CameraConfig>,
}

#[derive(Debug, Deserialize)]
struct CameraConfig {
    name: String,
    url: String,
}

fn read_config(path: impl AsRef<Path>) -> Result<Config, Box<dyn Error>> {
    let contents = fs::read_to_string(path)?;
    Ok(serde_yaml::from_str(&contents)?)
}
fn read_camera_list(path: impl AsRef<Path>) -> Result<CamerasCfg, Box<dyn Error>> {
    let contents = fs::read_to_string(path)?;
    Ok(serde_yaml::from_str(&contents)?)
}

fn main() {
    let mut cameras = vec![
        //Camera::new("", ""),                                                                           //Camera::new("", ""),
    ];

    let config = read_config("config/global.yml").unwrap();
    let camera_config = read_camera_list("config/camera-fetcher.yml").unwrap();
    let save_path = &config.save_path;

    for cam_cfg in camera_config.cameras {
        cameras.push(Camera::new(&cam_cfg.url, &cam_cfg.name));
    }

    let paused = Arc::new(AtomicBool::new(config.fetching_paused_at_startup));
    let loop_paused = Arc::clone(&paused);
    let loop_save_path = save_path.clone();

    thread::spawn(move || {
        run_camera_loop(cameras, loop_save_path, loop_paused);
    });

    web::serve(paused).expect("Failed to start web server");
}

fn run_camera_loop(mut cameras: Vec<Camera>, save_path: PathBuf, paused: Arc<AtomicBool>) {
    let start = Instant::now();
    let study_interval = Duration::from_secs(3 * 24 * 60 * 60); // 3 days in minutes
    let endless_study = true; // ignores study interval making the script run continusly
    let snapshot_interval = Duration::from_secs(60 * 2); // every 2 minutes

    let mut pause_notified = false;
    let mut resume_notified = true;

    println!("endless_study: {}", endless_study);
    println!("paused at startup: {}", paused.load(Ordering::Relaxed));

    loop {
        while paused.load(Ordering::Relaxed) {
            if !pause_notified {
                println!("PAUSED");
                pause_notified = true;
                resume_notified = false;
            }
            thread::sleep(Duration::from_millis(2000));
        }

        let loop_start = Instant::now();

        for cam_index in 0..cameras.len() {
            if paused.load(Ordering::Relaxed) {
                break;
            } else {
                if !resume_notified {
                    println!("RESUMED");
                    resume_notified = true;
                    pause_notified = false;
                }
            }

            println!("\nRunning task at {:?}", loop_start);
            let result = cameras[cam_index].fetch_image(save_path.to_str().unwrap());
            if let Ok(new) = result {
                cameras[cam_index].last_downloaded = new;
            }
        }

        let elapsed = loop_start.elapsed();
        if !endless_study && start.elapsed() > study_interval {
            println!("\nENDED STUDY INTERVAL");
            break;
        }

        let wait_until = Instant::now() + snapshot_interval.saturating_sub(elapsed);
        while Instant::now() < wait_until {
            while paused.load(Ordering::Relaxed) {
                thread::sleep(Duration::from_millis(250));
            }
            thread::sleep(Duration::from_millis(250).min(wait_until - Instant::now()));
        }
    }
}
