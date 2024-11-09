use crate::{watcher::player, Player};
use tokio::{
    sync::mpsc::{Receiver, Sender},
    task,
};

pub async fn task_spawner(
    mut task_spawner_event_handler_reciever: Receiver<(String, u16)>,
    task_spawner_exit_handler_sender: Sender<(
        String,
        Result<std::process::ExitStatus, std::io::Error>,
    )>,
    player: Player,
) {
    while let Some((streamer_name, quality)) = task_spawner_event_handler_reciever.recv().await {
        let stream = format!("https://www.twitch.tv/{}", streamer_name.clone());
        let player_func = player::get_stream(player, stream, quality).await;
        let sender_clone = task_spawner_exit_handler_sender.clone();
        task::spawn(async move {
            sender_clone
                .send((streamer_name, player_func.await))
                .await
                .unwrap()
        });
    }
}
pub async fn exit_handler(
    mut exit_handler_task_spawner_reciever: Receiver<(
        String,
        Result<std::process::ExitStatus, std::io::Error>,
    )>,
    exit_handler_event_handler_sender: Sender<(String, String)>,
) {
    while let Some((stream_name, result)) = exit_handler_task_spawner_reciever.recv().await {
        match result {
            Ok(exit_status) => {
                if !exit_status.success() {
                    exit_handler_event_handler_sender
                        .send((String::from("retry"), stream_name))
                        .await
                        .unwrap();
                }
            }
            Err(error) => {
                eprintln!("Error starting stream: {}", error);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::{process::Command, sync::mpsc, task};

    // #[tokio::test]
    // async fn spawn_task() {}

    #[tokio::test]
    async fn handle_good_exit() {
        let (process_sender, process_reciever) = mpsc::channel(10);
        let (exit_sender, exit_reciever) = mpsc::channel(10);

        task::spawn(async {
            exit_handler(process_reciever, exit_sender).await;
        });

        process_sender
            .send((String::from("kaicenat"), Command::new("ls").status().await))
            .await
            .unwrap();

        assert!(exit_reciever.is_empty());
    }

    #[tokio::test]
    async fn handle_bad_exit() {
        let (process_sender, process_reciever) = mpsc::channel(10);
        let (exit_sender, mut exit_reciever) = mpsc::channel(10);

        task::spawn(async {
            exit_handler(process_reciever, exit_sender).await;
        });

        process_sender
            .send((
                String::from("kaicenat"),
                Command::new("ls").arg("nonexistent").status().await,
            ))
            .await
            .unwrap();

        assert_eq!(
            exit_reciever.recv().await,
            Some((String::from("retry"), String::from("kaicenat")))
        );
    }

    // TODO: when websocket is implemented add test
    // #[tokio::test]
    // async fn handle_no_internet_exit() {}
}
