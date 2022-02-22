use chrono::prelude::*;
use chrono::Duration;
use ffprobe::{ffprobe, Format, Stream};
use serde::{Deserialize, Serialize};
use std::{fs::metadata, process, time, time::UNIX_EPOCH};

mod arg_parse;
mod config;
// mod folder;
mod json_reader;
mod playlist;

pub use arg_parse::get_args;
pub use config::{get_config, Config};
// pub use folder::walk;
pub use json_reader::{read_json, Program};
pub use playlist::program;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MediaProbe {
    pub format: Option<Format>,
    pub audio_streams: Option<Vec<Stream>>,
    pub video_streams: Option<Vec<Stream>>,
}

impl MediaProbe {
    fn new(input: String) -> Self {
        let probe = ffprobe(&input);
        let mut a_stream: Vec<Stream> = vec![];
        let mut v_stream: Vec<Stream> = vec![];

        match probe {
            Ok(obj) => {
                for stream in obj.streams {
                    let cp_stream = stream.clone();
                    match cp_stream.codec_type {
                        Some(codec_type) => {
                            if codec_type == "audio" {
                                a_stream.push(stream)
                            } else if codec_type == "video" {
                                v_stream.push(stream)
                            }
                        }

                        _ => {
                            println!("No codec type found for stream: {:?}", &stream)
                        }
                    }
                }

                MediaProbe {
                    format: Some(obj.format),
                    audio_streams: if a_stream.len() > 0 {
                        Some(a_stream)
                    } else {
                        None
                    },
                    video_streams: if v_stream.len() > 0 {
                        Some(v_stream)
                    } else {
                        None
                    },
                }
            }
            Err(err) => {
                println!(
                    "Can't read source '{}' with ffprobe, source is probably damaged! Error is: {:?}",
                    input,
                    err
                );

                MediaProbe {
                    format: None,
                    audio_streams: None,
                    video_streams: None,
                }
            }
        }
    }
}

// pub fn get_timestamp() -> i64 {
//     let local: DateTime<Local> = Local::now();

//     local.timestamp_millis() as i64
// }

pub fn get_sec() -> f64 {
    let local: DateTime<Local> = Local::now();

    (local.hour() * 3600 + local.minute() * 60 + local.second()) as f64
        + (local.nanosecond() as f64 / 1000000000.0)
}

pub fn get_date(seek: bool, start: f64, next: f64) -> String {
    let local: DateTime<Local> = Local::now();

    if seek && start > get_sec() {
        return (local - Duration::days(1)).format("%Y-%m-%d").to_string();
    }

    if start == 0.0 && next >= 86400.0 {
        return (local + Duration::days(1)).format("%Y-%m-%d").to_string();
    }

    local.format("%Y-%m-%d").to_string()
}

pub fn modified_time(path: String) -> Option<DateTime<Local>> {
    let metadata = metadata(path).unwrap();

    if let Ok(time) = metadata.modified() {
        let date_time: DateTime<Local> = time.into();
        return Some(date_time);
    }

    None
}

pub fn time_to_sec(time_str: &String) -> f64 {
    if ["now", "", "none"].contains(&time_str.as_str()) || !time_str.contains(":") {
        return 0.0
    }

    let t: Vec<&str> = time_str.split(':').collect();
    let h: f64 = t[0].parse().unwrap();
    let m: f64 = t[1].parse().unwrap();
    let s: f64 = t[2].parse().unwrap();

    h * 3600.0 + m * 60.0 + s
}

pub fn sec_to_time(sec: f64) -> String {
    let d = UNIX_EPOCH + time::Duration::from_secs(sec as u64);
    // Create DateTime from SystemTime
    let date_time = DateTime::<Utc>::from(d);

    date_time.format("%H:%M:%S").to_string()
}

pub fn is_close(a: f64, b: f64, to: f64) -> bool {
    if (a - b).abs() < to {
        return true;
    }

    false
}

pub fn get_delta(begin: &f64, config: &Config) -> f64 {
    let mut current_time = get_sec();
    let start = time_to_sec(&config.playlist.day_start);
    let length = time_to_sec(&config.playlist.length);
    let mut target_length = 86400.0;

    if length > 0.0 && length != target_length {
        target_length = length
    }

    if begin == &start && start == 0.0 && 86400.0 - current_time < 4.0 {
        current_time -= target_length
    } else if start >= current_time && begin != &start {
        current_time += target_length
    }

    let mut current_delta = begin - current_time;

    if is_close(current_delta, 86400.0, config.general.stop_threshold) {
        current_delta -= 86400.0
    }

    current_delta
}

pub fn check_sync(delta: f64, config: &Config) {
    if delta.abs() > config.general.stop_threshold && config.general.stop_threshold > 0.0 {
        println!("Start time out of sync for '{}' seconds", delta);
        process::exit(0x0100);
    }
}

pub fn gen_dummy(duration: f64, config: &Config) -> (String, Vec<String>) {
    let color = "#121212";
    let source = format!(
        "color=c={color}:s={}x{}:d={duration}",
        config.processing.width, config.processing.height
    );
    let cmd: Vec<String> = vec![
        "-f".to_string(),
        "lavfi".to_string(),
        "-i".to_string(),
        format!(
            "{source}:r={},format=pix_fmts=yuv420p",
            config.processing.fps
        ),
        "-f".to_string(),
        "lavfi".to_string(),
        "-i".to_string(),
        format!("anoisesrc=d={duration}:c=pink:r=48000:a=0.05"),
    ];

    (source, cmd)
}
