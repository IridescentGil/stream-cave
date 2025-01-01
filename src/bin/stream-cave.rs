use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};
use stream_cave::{create_oauth_token, Streams};

#[derive(Parser)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Manipulate oauth token
    Token(TokenArgs),
    /// Add/Edit/Remove streams
    Stream(StreamArgs),
    /// Play certain streams
    Play(PlayArgs),
}

#[derive(Args)]
struct TokenArgs {
    #[command(subcommand)]
    action: Option<TokenActions>,
}

#[derive(Subcommand)]
enum TokenActions {
    Create(TokenActionArgs),
    Delete(TokenActionArgs),
}

#[derive(Args)]
struct TokenActionArgs {
    config: Option<PathBuf>,
}
#[derive(Args)]
struct StreamArgs {
    #[command(subcommand)]
    action: StreamActions,
    #[arg(short, long)]
    config: Option<PathBuf>,
}

#[derive(Subcommand)]
enum StreamActions {
    Add(AddArgs),
    Remove(RemoveArgs),
    Edit(EditArgs),
    List,
}

#[derive(Args)]
struct AddArgs {
    name: String,
    quality_overrides: Option<Vec<String>>,
}

#[derive(Args)]
struct RemoveArgs {
    name: String,
}

#[derive(Args)]
struct EditArgs {
    name: String,
    quality_overrides: Option<Vec<String>>,
}

#[derive(Args)]
struct PlayArgs {
    stream: String,
    quality: Option<String>,
}

const CLIENT_ID: &str = "uty2ua26tqh28rzn3jketggzu98t6b";
const SEARCH_CHANNEL_API: &str = "https://api.twitch.tv/helix/search/channels";

#[tokio::main]
async fn main() {
    let args = Cli::parse();
    let system_paths = directories::ProjectDirs::from("com", "Iridescent", "Stream Watcher");

    match &args.command {
        Commands::Play(play) => {
            let player = stream_cave::Player::Mpv;
            let website = "https://www.twitch.tv/";
            let stream = format!("{}{}", website, play.stream.clone());
            if let Some(quality) = &play.quality {
                match quality.parse::<u16>() {
                    Ok(number) => {
                        stream_cave::get_stream(player, stream, number)
                            .await
                            .await
                            .expect("Unable to play stream");
                    }
                    Err(_) => {
                        if quality == "audio" {
                            stream_cave::get_stream(player, stream, 0)
                                .await
                                .await
                                .expect("Unable to play stream");
                        } else {
                            eprintln!("Please enter a number for stream quality or \"audio\" for audio only");
                        }
                    }
                }
            } else {
                stream_cave::get_stream(player, stream, 1080)
                    .await
                    .await
                    .expect("Unable to play stream");
            }
        }
        Commands::Token(token) => {
            if let Some(action) = &token.action {
                match action {
                    TokenActions::Create(arg) => {
                        if let Some(path) = &arg.config {
                            create_oauth_token(CLIENT_ID, path.as_path())
                                .await
                                .expect("Error when creating token");
                        } else if let Some(path) = system_paths {
                            create_oauth_token(CLIENT_ID, path.config_dir())
                                .await
                                .expect("Error when creating token");
                        }
                    }
                    TokenActions::Delete(arg) => {
                        if let Some(path) = &arg.config {
                            std::fs::remove_file(path.join("user-data.json"))
                                .expect("Unable to delete file");
                        } else if let Some(path) = system_paths {
                            std::fs::remove_file(path.config_dir().join("user-data.json"))
                                .expect("Unable to delete file");
                        }
                    }
                }
            }
        }
        Commands::Stream(stream) => {
            let config_option = stream.config.clone().unwrap_or_else(|| {
                system_paths
                    .as_ref()
                    .expect("Unable to enumerate system paths")
                    .config_dir()
                    .to_path_buf()
            });
            let mut schedule = Streams::read_streams(&config_option);

            match &stream.action {
                StreamActions::Add(action) => {
                    let mut user_access_token: Option<twitch_oauth2::tokens::UserToken> = None;
                    if let Err(error) = stream_cave::authentication::validate_oauth_token(
                        &mut user_access_token,
                        &config_option,
                        false,
                    )
                    .await
                    {
                        eprintln!("Error {error}.\nPlease retry creating a token.");
                        return;
                    }
                    schedule
                        .add_stream(
                            &action.name,
                            &action.quality_overrides,
                            SEARCH_CHANNEL_API,
                            CLIENT_ID,
                            user_access_token.expect("Expected to find token but found nothing"),
                        )
                        .await
                        .unwrap_or_else(|error| {
                            eprintln!("Error while performing operation: {error}");
                        });
                    schedule.write(&config_option).unwrap_or_else(|error| {
                        eprintln!("Error while performing operation: {error}");
                    });
                }
                StreamActions::Edit(action) => {
                    schedule
                        .edit_stream(&action.name, &action.quality_overrides)
                        .unwrap_or_else(|error| {
                            eprintln!("Error while performing operation: {error}");
                        });
                    schedule.write(&config_option).unwrap_or_else(|error| {
                        eprintln!("Error while performing operation: {error}");
                    });
                }
                StreamActions::Remove(action) => {
                    if schedule.remove_stream(&action.name).is_none() {
                        eprintln!("Streamer does not exist in file");
                    }
                    schedule.write(&config_option).unwrap_or_else(|error| {
                        eprintln!("Error while performing operation: {error}");
                    });
                }
                StreamActions::List => {
                    println!("{schedule}");
                }
            }
        }
    };
}
