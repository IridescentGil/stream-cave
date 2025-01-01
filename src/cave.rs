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
    /// use stream_cave::{Player, Settings};
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
    /// use stream_cave::Settings;
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
    streams: Vec<StreamConfig>,
}

impl Streams {
    /// Create new empty stream settings
    ///
    /// # Examples
    /// ```
    /// use stream_cave::Streams;
    ///
    /// let streams = Streams::new();
    /// ```
    #[must_use]
    pub const fn new() -> Self {
        Self {
            streams: Vec::new(),
        }
    }

    /// Read stream settings from file in directory
    ///
    /// # Examples
    /// ```no_run
    /// use stream_cave::Streams;
    /// use std::path::Path;
    ///
    /// let path = Path::new("./");
    /// let streams = Streams::read_streams(&path);
    /// ```
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

    /// Write data in struct to file
    ///
    /// # Errors
    /// Will return an error if the Streams struct cannot be serialized, if the cannot be created
    /// or the file cannot be written to.
    ///
    /// # Examples
    /// ```no_run
    /// use std::path::Path;
    /// use stream_cave::Streams;
    ///
    /// let path = Path::new("./");
    /// let streams = Streams::new();
    /// streams.write(&path).unwrap();
    /// ```
    pub fn write(&self, path: &Path) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let data = serde_json::to_string(self)?;
        match std::fs::create_dir(path) {
            Ok(()) => {
                let local_path = path.join("schedule.json");
                std::fs::write(&local_path, data)?;
            }
            Err(ref error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                let local_path = path.join("schedule.json");
                std::fs::write(&local_path, data)?;
            }
            Err(error) => return Err(error.into()),
        }
        Ok(())
    }

    /// Add a new streamer to the struct
    ///
    /// # Errors
    /// Will return an error on failure of the request to get the stream ID, failure to Deserialize
    /// the search results, and on failure to parse the id of the streamer.
    ///
    /// # Panics
    /// Will panic when failing to parse a profile quality.
    ///
    /// # Examples
    /// ```no_run
    /// use stream_cave::Streams;
    /// use stream_cave::authentication::validate_oauth_token;
    /// use twitch_oauth2::UserToken;
    /// use std::path::Path;
    ///
    /// #[tokio::main]
    /// async fn main() {
    /// const API_SEARCH_URL: &str = "https://api.twitch.tv/helix/search/channels";
    /// const CLIENT_ID: &str = "AAAAAAAAAAAA";
    /// let path = Path::new("./");
    /// let mut token = None;
    /// validate_oauth_token(&mut token,
    /// &path).await.unwrap();
    ///
    /// let mut streams = Streams::new();
    ///
    /// streams.add_stream("kaicenat", &None, API_SEARCH_URL, CLIENT_ID, token.unwrap()).await.unwrap();
    /// }
    /// ```
    pub async fn add_stream(
        &mut self,
        name: &str,
        quality_overides: &Option<Vec<String>>,
        api_url: &str,
        client_id: &str,
        user_access_token: UserToken,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut id = 0;
        let request = reqwest::Client::new()
            .get(api_url)
            .query(&[("query", name)])
            .bearer_auth(user_access_token.access_token.as_str())
            .header("Client-Id", client_id)
            .send()
            .await?;

        if request.status() == 200 {
            let search_results = request
                .json::<crate::twitch_socket::api_structs::StreamSearch>()
                .await?;

            let Some(stream_status) = search_results
                .data
                .iter()
                .find(|data_set| data_set.broadcaster_login == name)
            else {
                eprintln!("Unable to find streamer.");
                return Ok(());
            };
            id = stream_status.id.parse::<u32>()?;
        };

        let quality_overides = quality_overides
            .as_ref()
            .map_or_else(Vec::new, |overrides| {
                overrides
                    .iter()
                    .map(|to_parse| {
                        let overrides_split: Vec<&str> = to_parse.split(',').collect();
                        let profile = overrides_split[0].to_string();
                        let quality = overrides_split[1]
                            .parse::<u16>()
                            .expect("Unable to parse quality");
                        (profile, quality)
                    })
                    .collect()
            });

        self.streams.push(StreamConfig {
            name: name.to_string(),
            id,
            quality_overides,
            streams_to_open_on: Vec::new(),
            streams_to_close_on: Vec::new(),
        });
        Ok(())
    }

    /// Edit the settings of a certain stream
    ///
    /// # Errors
    /// Will return an error if it canot parse the `quality_overides` vector.
    ///
    /// # Examples
    /// ```no_run
    /// use stream_cave::Streams;
    /// use stream_cave::authentication::validate_oauth_token;
    /// use twitch_oauth2::UserToken;
    /// use std::path::Path;
    ///
    /// #[tokio::main]
    /// async fn main() {
    /// const API_SEARCH_URL: &str = "https://api.twitch.tv/helix/search/channels";
    /// const CLIENT_ID: &str = "AAAAAAAAAAAA";
    /// let path = Path::new("./");
    /// let mut token = None;
    /// validate_oauth_token(&mut token,
    /// &path).await.unwrap();
    ///
    /// let mut streams = Streams::new();
    ///
    /// streams.add_stream("kaicenat", &None, API_SEARCH_URL, CLIENT_ID, token.unwrap()).await.unwrap();
    ///
    /// streams.edit_stream("kaicenat", &Some(vec![String::from("normal,720")])).unwrap();
    /// }
    /// ```
    pub fn edit_stream(
        &mut self,
        name: &str,
        quality_overides: &Option<Vec<String>>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if let Some(overrides) = quality_overides {
            let profile_overrides: Vec<(String, u16)> = overrides
                .iter()
                .map(|to_parse| {
                    let overrides_split: Vec<&str> = to_parse.split(',').collect();
                    let profile = overrides_split[0].to_string();
                    match overrides_split[1].parse::<u16>() {
                        Ok(quality) => Ok((profile, quality)),
                        Err(error) => Err(error),
                    }
                })
                .collect::<Result<Vec<_>, _>>()?;

            if let Some(streamer) = self.streams.iter_mut().find(|config| config.name == name) {
                for profile_override in profile_overrides {
                    if let Some(found_profile) = streamer
                        .quality_overides
                        .iter_mut()
                        .find(|(profile, _)| *profile == profile_override.0)
                    {
                        *found_profile = profile_override;
                    } else {
                        streamer.quality_overides.push(profile_override);
                    }
                }
            }
        }
        Ok(())
    }

    /// Remove a stream
    ///
    /// # Examples
    /// ```no_run
    /// use stream_cave::Streams;
    /// use std::path::Path;
    ///
    /// let path = Path::new("./");
    /// let mut streams = Streams::read_streams(&path);
    ///
    /// streams.remove_stream("kaicenat").unwrap();
    /// ```
    pub fn remove_stream(&mut self, name: &str) -> Option<StreamConfig> {
        match self
            .streams
            .iter()
            .position(|stream_config| stream_config.name == name)
        {
            Some(position) => Some(self.streams.remove(position)),
            None => None,
        }
    }
}

impl Default for Streams {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for Streams {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        for streams in &self.streams {
            write!(f, "{streams}")?;
        }
        Ok(())
    }
}

/// Individual twitch stream settings
#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, Clone)]
pub struct StreamConfig {
    pub name: String,
    pub id: u32,
    pub quality_overides: Vec<(String, u16)>,
    pub streams_to_close_on: Vec<String>,
    pub streams_to_open_on: Vec<String>,
}

impl std::fmt::Display for StreamConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}:\n  profile quality:\n", self.name)?;
        for (profile, quality) in &self.quality_overides {
            writeln!(f, "    {profile} : {quality}")?;
        }
        writeln!(f, "  streams to close on:")?;
        for close_on_streams in &self.streams_to_close_on {
            writeln!(f, "    {close_on_streams}")?;
        }
        writeln!(f, "  streams to open on:")?;
        for open_on_streams in &self.streams_to_open_on {
            writeln!(f, "    {open_on_streams}")?;
        }

        Ok(())
    }
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
    ///
    /// # Errors
    /// Will return an error if it cannot find or open the file in path, and if the file cannot be
    /// deserialized.
    ///
    /// # Examples
    /// ```no_run
    /// use std::path::Path;
    /// use stream_cave::UserData;
    ///
    /// let path = Path::new("./user-data.json");
    /// let data = UserData::from_file(&path).unwrap();
    /// ```
    pub fn from_file(path: &Path) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let file = read_to_string(path)?;
        Ok(serde_json::from_str(&file)?)
    }
    /// Create `UserData` from `UserToken`
    ///
    /// # Examples
    /// ```no_run
    /// use stream_cave::UserData;
    /// use stream_cave::authentication::validate_oauth_token;
    /// use twitch_oauth2::UserToken;
    /// use std::path::Path;
    ///
    /// #[tokio::main]
    /// async fn main() {
    /// let path = Path::new("./");
    /// let mut token = None;
    /// validate_oauth_token(&mut token,
    /// &path).await.unwrap();
    ///
    /// let user_data = UserData::from_token(token.as_ref().unwrap());
    /// }
    /// ```
    #[must_use]
    pub fn from_token(token: &UserToken) -> Self {
        Self {
            access_token: token.access_token.secret().to_string(),
            login: token.login.to_string(),
            user_id: token.user_id.to_string(),
        }
    }
    /// Write token to file
    ///
    /// # Errors
    /// Will return an error if the data cannot be serialized or if it cannot be written to the
    /// file in path.
    ///
    /// # Examples
    /// ```no_run
    /// use stream_cave::UserData;
    /// use stream_cave::authentication::validate_oauth_token;
    /// use twitch_oauth2::UserToken;
    /// use std::path::Path;
    ///
    /// #[tokio::main]
    /// async fn main() {
    /// let path = Path::new("./");
    /// let mut token = None;
    /// validate_oauth_token(&mut token,
    /// &path).await.unwrap();
    ///
    /// let user_data = UserData::from_token(token.as_ref().unwrap());
    ///
    /// user_data.save(&path.join("user_data.json")).unwrap();
    /// }
    /// ```
    pub fn save(&self, path: &Path) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let token_string = serde_json::to_string(self)?;
        std::fs::write(path, token_string)?;
        Ok(())
    }
}
