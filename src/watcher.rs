pub mod authentication;
pub mod event_handler;
pub mod file_watcher;
pub mod player;
pub mod tasks_handler;
pub mod twitch_socket;

use serde::{Deserialize, Serialize};
use std::{
    fs::read_to_string,
    path::{Path, PathBuf},
};
use twitch_oauth2::UserToken;

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum Player {
    Mpv,
    Streamlink,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Settings {
    ver: (u8, u8),
    pub player: Player,
    pub schedule: PathBuf,
    pub profile: (String, u16),
}

impl Settings {
    pub fn new(path: PathBuf) -> Self {
        let ver = (0, 1);
        let player = Player::Mpv;
        let schedule = path;
        let profile = (String::from("normal"), 1080);

        Settings {
            ver,
            player,
            schedule,
            profile,
        }
    }
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

#[derive(Deserialize, Serialize)]
pub struct UserData {
    pub access_token: String,
    pub login: String,
    pub user_id: String,
}

impl UserData {
    fn from_file(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let file = read_to_string(path)?;
        Ok(serde_json::from_str(&file)?)
    }
    fn from_token(token: &UserToken) -> Self {
        Self {
            access_token: token.access_token.secret().to_string(),
            login: token.login.to_string(),
            user_id: token.user_id.to_string(),
        }
    }
    fn save(&self, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        let token_string = serde_json::to_string(self)?;
        std::fs::write(path, token_string)?;
        Ok(())
    }
}
