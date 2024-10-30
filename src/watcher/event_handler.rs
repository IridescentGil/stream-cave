use tokio::sync::mpsc::{Receiver, Sender};

use super::StreamConfig;

pub async fn event_handler(
    mut event_handler_twitch_websocket_reciever: Receiver<(String, String)>,
    mut event_handler_exit_handler_reciever: Receiver<(String, String)>,
    mut event_handler_file_watcher_reciever: Receiver<StreamConfig>,
    event_handler_task_spawner_sender: Sender<(String, u16)>,
) {
}
