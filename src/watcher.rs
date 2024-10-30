pub mod event_handler;
pub mod file_watcher;
pub mod player;
pub mod tasks_handler;
pub mod twitch_socket;

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

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

#[derive(Serialize, Deserialize, Debug)]
pub struct Streams {
    names: Vec<(String, u32)>,
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

#[derive(Debug, PartialEq)]
pub struct StreamConfig {
    pub name: String,
    pub id: u32,
    pub quality_overides: Vec<(String, u16)>,
    pub streams_to_close_on: Vec<String>,
    pub streams_to_open_on: Vec<String>,
}
