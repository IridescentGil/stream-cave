mod watcher;
use crate::watcher::*;
use std::path::{Path, PathBuf};
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
        std::fs::create_dir(&local_path).unwrap();

        local_path.push("config.json");

        std::fs::write(local_path, data).unwrap();
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
pub async fn run(settings: Settings, streams: Streams) {
    let (file_watcher_twitch_websocket_sender, twitch_socket_file_watcher_reciever) =
        mpsc::channel(10);
    let (file_watcher_event_handler_sender, event_handler_file_watcher_reciever) =
        mpsc::channel(10);
    let (twitch_websocket_event_handler_sender, event_handler_twitch_websocket_reciever) =
        mpsc::channel(10);
    let (event_handler_task_spawner_sender, task_spawner_event_handler_reciever) =
        mpsc::channel(10);
    let (task_spawner_exit_handler_sender, exit_handler_task_spawner_reciever) = mpsc::channel(10);
    let (exit_handler_event_handler_sender, event_handler_exit_handler_reciever) =
        mpsc::channel(10);

    let (first_task, second_task, third_task, fourth_task, fifth_task) = tokio::join!(
        task::spawn(async move {
            file_watcher::file_watcher(
                file_watcher_twitch_websocket_sender,
                file_watcher_event_handler_sender,
                &settings.schedule,
                streams,
            )
            .await
        }),
        task::spawn(async move {
            twitch_socket::twitch_websocket(
                twitch_socket_file_watcher_reciever,
                twitch_websocket_event_handler_sender,
            )
            .await
        }),
        task::spawn(async move {
            event_handler::event_handler(
                event_handler_twitch_websocket_reciever,
                event_handler_exit_handler_reciever,
                event_handler_file_watcher_reciever,
                event_handler_task_spawner_sender,
            )
            .await
        }),
        task::spawn(async move {
            tasks_handler::task_spawner(
                task_spawner_event_handler_reciever,
                task_spawner_exit_handler_sender,
                settings.player,
            )
            .await
        }),
        task::spawn(async move {
            tasks_handler::exit_handler(
                exit_handler_task_spawner_reciever,
                exit_handler_event_handler_sender,
            )
            .await
        })
    );

    if let Err(error) = first_task {
        eprintln!("Error encountered in task: {}", error);
    }
    if let Err(error) = second_task {
        eprintln!("Error encountered in task: {}", error);
    }
    if let Err(error) = third_task {
        eprintln!("Error encountered in task: {}", error);
    }
    if let Err(error) = fourth_task {
        eprintln!("Error encountered in task: {}", error);
    }
    if let Err(error) = fifth_task {
        eprintln!("Error encountered in task: {}", error);
    }
}
