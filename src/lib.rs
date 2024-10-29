mod watcher;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub enum Player {
    Mpv,
    Streamlink,
}

#[derive(Debug)]
pub struct Settings {
    pub player: Player,
    pub schedule: PathBuf,
}

use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize, Debug)]
pub struct Streams {
    names: Vec<String>,
    quality_overides: Vec<Vec<(String, u16)>>,
    streams_to_close_on: Vec<Vec<String>>,
    streams_to_open_on: Vec<Vec<String>>,
}

impl Streams {
    pub fn new() -> Self {
        Self {
            names: Vec::new(),
            quality_overides: Vec::new(),
            streams_to_close_on: Vec::new(),
            streams_to_open_on: Vec::new(),
        }
    }
}

impl Default for Streams {
    fn default() -> Self {
        Self::new()
    }
}

pub fn read_config(_flags: Vec<String>, paths: Vec<PathBuf>) -> Settings {
    let mut player = Player::Mpv;
    // TODO: Split schedule finder from config finder
    let mut schedule: PathBuf = PathBuf::new();

    for path in paths {
        if path.as_path().join("config.conf").exists() {
            let config = std::fs::read_to_string(path.join("config.conf"));
            match config {
                Ok(settings) => {
                    schedule = path;
                    for line in settings.lines() {
                        if line.contains("player:") {
                            if line.contains("mpv") {
                                player = Player::Mpv;
                            } else if line.contains("streamlink") {
                                player = Player::Streamlink;
                            }
                        }
                    }
                }
                Err(error) => {
                    eprintln!("Error opening config file: {}", error);
                    return Settings {
                        player: Player::Mpv,
                        schedule: path,
                    };
                }
            }
        }
    }

    Settings { player, schedule }
}

pub fn read_streams(path: &Path) -> Streams {
    let file = std::fs::read_to_string(path.join("schedule.json"));
    match file {
        Ok(data) => match serde_json::from_str(&data) {
            Ok(json) => json,
            Err(error) => {
                eprintln!("Error deserializing data: {}", error);
                Streams::new()
            }
        },
        Err(error) => {
            eprint!("Error opening file: {}", error);
            Streams::new()
        }
    }
}
pub fn run(settings: Settings, streams: Streams) {
    println!("settings: {:?}, streams: {:?}", settings, streams);
    let player = settings.player;

    let player_func = crate::watcher::player::get_player_func(player);
}
