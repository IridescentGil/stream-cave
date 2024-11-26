mod watcher;
pub use crate::watcher::authentication::create_oauth_token;
use crate::watcher::*;
use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{sync::mpsc, task};

pub fn read_config(_flags: Vec<String>, paths: Vec<PathBuf>) -> Settings {
    let mut local_path = paths.last().unwrap_or(&PathBuf::new()).clone();

    for path in paths {
        if path.as_path().join("config.json").exists() {
            let config = std::fs::read_to_string(path.join("config.json"));
            match config {
                Ok(settings) => match serde_json::from_str(&settings) {
                    Ok(json) => return json,
                    Err(error) => {
                        eprintln!("Error deserializing data: {}", error);
                        return Settings::new(path);
                    }
                },
                Err(error) => {
                    eprintln!("Error opening config file: {}", error);
                    return Settings::new(path);
                }
            }
        }
    }
    let new_settings = Settings::new(local_path.clone());
    if let Ok(data) = serde_json::to_string(&new_settings) {
        std::fs::create_dir(&local_path).expect("Unable to create directory");

        local_path.push("config.json");

        std::fs::write(&local_path, data).unwrap_or_else(|_| {
            eprintln!(
                "Unable to write to file: {}",
                local_path.to_str().expect("Non UTF-8 String")
            );
        });
    }

    new_settings
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

pub const CLIENT_ID: &str = "uty2ua26tqh28rzn3jketggzu98t6b";

pub async fn run(settings: &Arc<Settings>, streams: &Arc<Mutex<Streams>>) {
    #[cfg(debug_assertions)]
    const TWITCH_WEBSOCKET_URL: &str = "ws://127.0.0.1:8080/ws";
    #[cfg(debug_assertions)]
    const TWITCH_API_URL: &str = "http://localhost:8080/eventsub/subscriptions";

    #[cfg(not(debug_assertions))]
    const TWITCH_WEBSOCKET_URL: &str = "wss://eventsub.wss.twitch.tv/ws";
    #[cfg(not(debug_assertions))]
    const TWITCH_API_URL: &str = "https://api.twitch.tv/helix/eventsub/subscriptions";

    const STREAMING_SITE: &str = "https://www.twitch.tv/";
    const SEARCH_CHANNEL_API: &str = "https://api.twitch.tv/helix/search/channels";

    loop {
        let mut token: Option<twitch_oauth2::tokens::UserToken> = None;
        loop {
            match authentication::validate_oauth_token(&mut token, &settings.schedule).await {
                Ok(_) => break,
                Err(error) => {
                    eprintln!(
                        "Error {}.\nPlease retry creating a token. Re-checking token in 60 seconds",
                        error
                    );
                    tokio::time::sleep(Duration::from_secs(60)).await;
                }
            };
        }
        let user_access_token = Arc::new(token);
        loop {
            let streams = streams.clone();
            let settings_player = settings.clone();
            let settings_path = settings.clone();

            //TODO: create function to delete inactive subscriptions
            let user_access_token_websocket = user_access_token.clone();
            let user_access_token_exit_handler = user_access_token.clone();

            let (file_watcher_twitch_websocket_sender, twitch_socket_file_watcher_reciever) =
                mpsc::channel(10);
            let (file_watcher_event_handler_sender, event_handler_file_watcher_reciever) =
                mpsc::channel(10);
            let (twitch_websocket_event_handler_sender, event_handler_twitch_websocket_reciever) =
                mpsc::channel(10);
            let (event_handler_task_spawner_sender, task_spawner_event_handler_reciever) =
                mpsc::channel(10);
            let (task_spawner_exit_handler_sender, exit_handler_task_spawner_reciever) =
                mpsc::channel(10);
            let (exit_handler_event_handler_sender, event_handler_exit_handler_reciever) =
                mpsc::channel(10);
            let (restart_signal_sender, mut restart_signal_reciever) = mpsc::channel(1);
            let restart_signal_sender_exit_handler = restart_signal_sender.clone();
            let restart_signal_sender_twitch_socket = restart_signal_sender.clone();

            tokio::select! {
               Err(error) = task::spawn(async move {
                    file_watcher::file_watcher(
                        file_watcher_twitch_websocket_sender,
                        file_watcher_event_handler_sender,
                        &settings_path.schedule,
                        &streams,
                    )
                    .await
                }) => {
                    eprintln!("Error encountered in task: {}", error);
                }
               Err(error) = task::spawn(async move {
                    twitch_socket::twitch_websocket(
                        twitch_socket_file_watcher_reciever,
                        twitch_websocket_event_handler_sender,
                        restart_signal_sender_twitch_socket,
                        TWITCH_WEBSOCKET_URL,
                        TWITCH_API_URL,
                        user_access_token_websocket,
                        CLIENT_ID,
                    )
                    .await
                }) => {
                    eprintln!("Error encountered in task: {}", error);
                }
               Err(error) = task::spawn(async move {
                    event_handler::event_handler(
                        event_handler_twitch_websocket_reciever,
                        event_handler_exit_handler_reciever,
                        event_handler_file_watcher_reciever,
                        event_handler_task_spawner_sender,
                    )
                    .await
                }) => {
                    eprintln!("Error encountered in task: {}", error);
                }
               Err(error) = task::spawn(async move {
                    tasks_handler::task_spawner(
                        task_spawner_event_handler_reciever,
                        task_spawner_exit_handler_sender,
                        settings_player.player,
                        STREAMING_SITE.to_string(),
                    )
                    .await
                }) => {
                    eprintln!("Error encountered in task: {}", error);
                }
               Err(error) = task::spawn(async move {
                    tasks_handler::exit_handler(
                        exit_handler_task_spawner_reciever,
                        exit_handler_event_handler_sender,
                        restart_signal_sender_exit_handler,
                        SEARCH_CHANNEL_API.to_string(),
                        user_access_token_exit_handler,
                        CLIENT_ID,
                    )
                    .await
                }) => {
                    eprintln!("Error encountered in task: {}", error);
                }
                Some(code) = restart_signal_reciever.recv() => {
                    println!("Code restart code: {}", code);
                    match code{
                        1 => continue,
                        2 => break,
                        _ => println!("Unrecognized code"),
                    }
                }
            };
        }
    }
}
