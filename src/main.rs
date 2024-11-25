use std::{path::PathBuf, sync::Arc};

use std::sync::Mutex;

#[tokio::main]
async fn main() {
    let flags: Vec<String> = std::env::args().collect();
    let mut paths: Vec<PathBuf> = Vec::new();

    paths.push(PathBuf::from("./"));
    if let Some(dirs) = directories::ProjectDirs::from("com", "Iridescent", "Stream Watcher") {
        paths.push(dirs.config_local_dir().to_owned());
        paths.push(dirs.config_dir().to_owned());
    }

    let settings = Arc::new(stream_watcher::read_config(flags, paths));
    let streams = Arc::new(Mutex::new(stream_watcher::read_streams(&settings.schedule)));
    stream_watcher::run(&settings, &streams).await;
}
