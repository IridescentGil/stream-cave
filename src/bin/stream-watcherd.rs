use clap::Parser;
use std::{path::PathBuf, sync::Arc};

use std::sync::Mutex;

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

    let settings = Arc::new(stream_watcher::read_config(paths));
    let streams = Arc::new(Mutex::new(stream_watcher::read_streams(&settings.schedule)));
    stream_watcher::run(&settings, &streams).await;
}
