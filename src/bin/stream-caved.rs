use clap::Parser;
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Duration,
};
use stream_cave::{
    authentication, event_handler, file_watcher, tasks_handler, twitch_socket, Settings, Streams,
};
use tokio::{sync::mpsc, task};

const CLIENT_ID: &str = "uty2ua26tqh28rzn3jketggzu98t6b";

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Set directory of config file to use
    #[arg(short, long)]
    config: Option<PathBuf>,
}

#[tokio::main]
async fn main() {
    let flags = Args::parse();
    let mut paths: Vec<PathBuf> = Vec::new();

    if let Some(config_path) = flags.config {
        paths.push(config_path);
    } else if let Some(dirs) = directories::ProjectDirs::from("com", "Iridescent", "Stream Watcher")
    {
        paths.push(dirs.config_local_dir().to_owned());
        paths.push(dirs.config_dir().to_owned());
    }

    let settings =
        Arc::new(Settings::read_config(&paths).expect("Unable to create new config file"));
    let streams = Arc::new(Mutex::new(Streams::read_streams(&settings.schedule)));
    run(&settings, &streams).await;
}

async fn run(settings: &Arc<Settings>, streams: &Arc<Mutex<Streams>>) {
    const TWITCH_WEBSOCKET_URL: &str = "wss://eventsub.wss.twitch.tv/ws";
    const TWITCH_API_URL: &str = "https://api.twitch.tv/helix/eventsub/subscriptions";
    const STREAMING_SITE: &str = "https://www.twitch.tv/";
    const SEARCH_CHANNEL_API: &str = "https://api.twitch.tv/helix/search/channels";

    loop {
        let mut token: Option<twitch_oauth2::tokens::UserToken> = None;
        loop {
            match authentication::validate_oauth_token(&mut token, &settings.schedule).await {
                Ok(()) => break,
                Err(error) => {
                    eprintln!(
                        "Error {error}.\nPlease retry creating a token. Re-checking token in 60 seconds"
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

            task::spawn(async move {
                file_watcher::file_watcher(
                    file_watcher_twitch_websocket_sender,
                    file_watcher_event_handler_sender,
                    &settings_path.schedule,
                    &streams,
                )
                .await;
            });
            task::spawn(async move {
                twitch_socket::twitch_websocket(
                    twitch_socket_file_watcher_reciever,
                    twitch_websocket_event_handler_sender,
                    restart_signal_sender_twitch_socket,
                    TWITCH_WEBSOCKET_URL,
                    TWITCH_API_URL,
                    user_access_token_websocket,
                    CLIENT_ID,
                );
            });
            task::spawn(async move {
                event_handler::event_handler(
                    event_handler_twitch_websocket_reciever,
                    event_handler_exit_handler_reciever,
                    event_handler_file_watcher_reciever,
                    event_handler_task_spawner_sender,
                );
            });
            task::spawn(async move {
                tasks_handler::task_spawner(
                    task_spawner_event_handler_reciever,
                    task_spawner_exit_handler_sender,
                    settings_player.player,
                    STREAMING_SITE.to_string(),
                )
                .await;
            });
            task::spawn(async move {
                tasks_handler::exit_handler(
                    exit_handler_task_spawner_reciever,
                    exit_handler_event_handler_sender,
                    restart_signal_sender_exit_handler,
                    SEARCH_CHANNEL_API.to_string(),
                    user_access_token_exit_handler,
                    CLIENT_ID,
                )
                .await;
            });

            if let Some(code) = restart_signal_reciever.recv().await {
                match code {
                    1 => continue,
                    2 => break,
                    _ => println!("Unrecognized code"),
                }
            } else {
                return;
            }
        }
    }
}
