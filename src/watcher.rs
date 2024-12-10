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

/// Which player process to use
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Player {
    Mpv,
    Streamlink,
}

/// The configuration settings of the program.
#[derive(Serialize, Deserialize, Debug)]
pub struct Settings {
    ver: (u8, u8),
    pub player: Player,
    pub schedule: PathBuf,
    pub profile: (String, u16),
}

impl Settings {
    /// Create a new settings struct with default settings.
    /// The path will point to the directory of the schedule.json
    ///
    /// # Examples
    /// ```
    /// use std::path::{Path, PathBuf};
    /// use stream_watcher::{Player, Settings};
    /// let settings = Settings::new(Path::new("./"));
    ///
    /// assert_eq!(Player::Mpv, settings.player);
    /// assert_eq!(PathBuf::from("./"), settings.schedule);
    /// assert_eq!((String::from("normal"), 1080), settings.profile);
    /// ```
    #[must_use]
    pub fn new(path: &Path) -> Self {
        let ver = (0, 1);
        let player = Player::Mpv;
        let schedule = path.to_path_buf();
        let profile = (String::from("normal"), 1080);

        Self {
            ver,
            player,
            schedule,
            profile,
        }
    }

    /// Populate settings with configurations from a file.
    /// Takes a slice of directories to look for config files.
    /// Creates a new config file in the last directory if no
    /// config files are found.
    /// # Examples
    /// ```no_run
    /// use std::path::PathBuf;
    /// use stream_watcher::Settings;
    ///
    /// let mut paths = Vec::new();
    /// paths.push(PathBuf::from("./"));
    ///
    /// let settings = Settings::read_config(&paths).unwrap();
    /// ```
    ///
    /// # Errors
    /// Will return an error if unable to create the config directory when creating new config file or
    /// if writing to the file fails
    pub fn read_config(paths: &[PathBuf]) -> std::io::Result<Self> {
        for path in paths {
            if path.as_path().join("config.json").exists() {
                let config = std::fs::read_to_string(path.join("config.json"));
                match config {
                    Ok(settings) => match serde_json::from_str(&settings) {
                        Ok(json) => return Ok(json),
                        Err(error) => {
                            eprintln!("Error deserializing data: {error}");
                            return Ok(Self::new(path));
                        }
                    },
                    Err(error) => {
                        eprintln!("Error opening config file: {error}");
                        return Ok(Self::new(path));
                    }
                }
            }
        }

        let local_path = paths.last().map_or_else(|| Path::new(""), |path| path);

        let new_settings = Self::new(local_path);
        if let Ok(data) = serde_json::to_string(&new_settings) {
            match std::fs::create_dir(local_path) {
                Ok(()) => {
                    let local_path = local_path.join("config.json");
                    std::fs::write(&local_path, data)?;
                }
                Err(ref error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                    let local_path = local_path.join("config.json");
                    std::fs::write(&local_path, data)?;
                }
                Err(error) => return Err(error),
            }
        }

        Ok(new_settings)
    }
}

/// Contains settings for the twitch streams to watch
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Streams {
    names: Vec<(String, u32)>,
    quality_overides: Vec<Vec<(String, u16)>>,
    streams_to_close_on: Vec<Vec<String>>,
    streams_to_open_on: Vec<Vec<String>>,
}

impl Streams {
    /// Create new empty stream settings
    #[must_use]
    pub const fn new() -> Self {
        Self {
            names: Vec::new(),
            quality_overides: Vec::new(),
            streams_to_close_on: Vec::new(),
            streams_to_open_on: Vec::new(),
        }
    }

    /// Read stream settings from file in directory
    #[must_use]
    pub fn read_streams(path: &Path) -> Self {
        let file = std::fs::read_to_string(path.join("schedule.json"));
        match file {
            Ok(data) => match serde_json::from_str(&data) {
                Ok(json) => json,
                Err(error) => {
                    eprintln!("Error deserializing data: {error}");
                    Self::new()
                }
            },
            Err(error) => {
                eprint!("Error opening file: {error}");
                Self::new()
            }
        }
    }
}

impl Default for Streams {
    fn default() -> Self {
        Self::new()
    }
}

/// Individual twitch stream settings
#[derive(Debug, PartialEq, Eq)]
pub struct StreamConfig {
    pub name: String,
    pub id: u32,
    pub quality_overides: Vec<(String, u16)>,
    pub streams_to_close_on: Vec<String>,
    pub streams_to_open_on: Vec<String>,
}

/// Twitch oauth2 token settings
#[derive(Deserialize, Serialize)]
pub struct UserData {
    pub access_token: String,
    pub login: String,
    pub user_id: String,
}

impl UserData {
    /// Read twitch token from storage
    fn from_file(path: &Path) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let file = read_to_string(path)?;
        Ok(serde_json::from_str(&file)?)
    }
    /// Create `UserData` from `UserToken`
    fn from_token(token: &UserToken) -> Self {
        Self {
            access_token: token.access_token.secret().to_string(),
            login: token.login.to_string(),
            user_id: token.user_id.to_string(),
        }
    }
    /// Write token to file
    fn save(&self, path: &Path) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let token_string = serde_json::to_string(self)?;
        std::fs::write(path, token_string)?;
        Ok(())
    }
}
