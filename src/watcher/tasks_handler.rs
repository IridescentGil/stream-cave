use crate::{watcher::player, Player};
use tokio::sync::mpsc::{Receiver, Sender};

pub async fn task_spawner(
    mut task_spawner_event_handler_reciever: Receiver<(String, u16)>,
    task_spawner_exit_handler_sender: Sender<Result<std::process::ExitStatus, std::io::Error>>,
    player: &Player,
) {
    let streamer = String::new();
    let player_func = player::get_stream(&player, &streamer, 480);
    let player_func2 = player::get_stream(&player, &streamer, 480);
}
pub async fn exit_handler(
    mut exit_handler_task_spawner_reciever: Receiver<
        Result<std::process::ExitStatus, std::io::Error>,
    >,
    exit_handler_event_handler_sender: Sender<(String, String)>,
) {
}
