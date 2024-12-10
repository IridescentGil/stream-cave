use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};
use stream_watcher::create_oauth_token;

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
}

#[derive(Subcommand)]
enum StreamActions {
    Add(AddArgs),
    Remove(RemoveArgs),
    Edit(EditArgs),
}

#[derive(Args)]
struct AddArgs {
    name: String,
}

#[derive(Args)]
struct RemoveArgs {
    name: String,
}

#[derive(Args)]
struct EditArgs {
    name: String,
}

#[derive(Args)]
struct PlayArgs {
    stream: String,
    quality: Option<String>,
}

const CLIENT_ID: &str = "uty2ua26tqh28rzn3jketggzu98t6b";

#[tokio::main]
async fn main() {
    let args = Cli::parse();

    match &args.command {
        Commands::Play(play) => {
            let player = stream_watcher::Player::Mpv;
            let website = "https://www.twitch.tv/";
            let stream = format!("{}{}", website, play.stream.clone());
            if let Some(quality) = &play.quality {
                match quality.parse::<u16>() {
                    Ok(number) => {
                        stream_watcher::get_stream(player, stream, number)
                            .await
                            .await
                            .unwrap();
                    }
                    Err(_) => {
                        if quality == "audio" {
                            stream_watcher::get_stream(player, stream, 0)
                                .await
                                .await
                                .unwrap();
                        } else {
                            eprintln!("Please enter a number for stream quality or \"audio\" for audio only");
                        }
                    }
                }
            } else {
                stream_watcher::get_stream(player, stream, 1080)
                    .await
                    .await
                    .unwrap();
            }
        }
        Commands::Token(token) => {
            if let Some(action) = &token.action {
                match action {
                    TokenActions::Create(arg) => {
                        if let Some(path) = &arg.config {
                            create_oauth_token(CLIENT_ID, path.as_path()).await.unwrap();
                        } else if let Some(path) =
                            directories::ProjectDirs::from("com", "Iridescent", "Stream Watcher")
                        {
                            create_oauth_token(CLIENT_ID, path.config_dir())
                                .await
                                .unwrap();
                        }
                    }
                    TokenActions::Delete(arg) => {}
                }
            }
        }
        Commands::Stream(stream) => match &stream.action {
            StreamActions::Add(action) => {}
            StreamActions::Edit(action) => {}
            StreamActions::Remove(action) => {}
        },
    };
}
