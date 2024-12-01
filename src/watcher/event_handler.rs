use std::sync::{Arc, Mutex};
use tokio::task::yield_now;
use tokio::{
    sync::mpsc::{Receiver, Sender},
    task,
};

use super::StreamConfig;

pub async fn event_handler(
    mut event_handler_twitch_websocket_reciever: Receiver<(String, String)>,
    mut event_handler_exit_handler_reciever: Receiver<(String, String)>,
    mut event_handler_file_watcher_reciever: Receiver<StreamConfig>,
    event_handler_task_spawner_sender: Sender<(String, u16)>,
) {
    let streamer_configs = Arc::new(Mutex::new(Vec::new()));
    let file_configs = streamer_configs.clone();
    let socket_configs = streamer_configs.clone();
    let exit_configs = streamer_configs.clone();

    let socket_sender = event_handler_task_spawner_sender.clone();
    let exit_sender = event_handler_task_spawner_sender.clone();

    // FIXME: get global profile from settings
    let global_profile = ("normal", 1080);

    task::spawn(async move {
        while let Some(config) = event_handler_file_watcher_reciever.recv().await {
            file_configs
                .lock()
                .as_mut()
                .expect("Mutex lock poisoned")
                .push(config);
        }
    });

    task::spawn(async move {
        while let Some(stream) = event_handler_twitch_websocket_reciever.recv().await {
            let sender_clone = socket_sender.clone();
            let config_clone = socket_configs.clone();
            handle_event(config_clone, stream, sender_clone, &global_profile).await;
        }
    });

    task::spawn(async move {
        while let Some(stream) = event_handler_exit_handler_reciever.recv().await {
            let sender_clone = exit_sender.clone();
            let config_clone = exit_configs.clone();
            handle_event(config_clone, stream, sender_clone, &global_profile).await;
        }
    });
}

async fn handle_event(
    configs: Arc<Mutex<Vec<StreamConfig>>>,
    stream: (String, String),
    sender: Sender<(String, u16)>,
    global_profile: &(&str, u16),
) {
    let mut stream_quality = global_profile.1;

    yield_now().await;
    let found_configurations = configs
        .lock()
        .as_ref()
        .expect("Mutex lock poisoned")
        .iter()
        .filter(|streamer| streamer.name == stream.1)
        .count();
    if found_configurations == 1 {
        let streamer_configs = configs.lock();
        let config = streamer_configs
            .as_ref()
            .expect("Mutex lock poisoned")
            .iter()
            .find(|streamer| streamer.name == stream.1)
            .unwrap();
        let global_quality_overrides = &config.quality_overides;
        if let Some(current_profile_override) = global_quality_overrides
            .iter()
            .find(|(profile, _)| *profile == global_profile.0)
        {
            stream_quality = current_profile_override.1;
        }
    }

    let task = (stream.1.clone(), stream_quality);
    sender.send(task).await.unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::{sync::mpsc, task};

    #[tokio::test]
    async fn no_config_event_handling() {
        let (socket_sender, socket_reciever) = mpsc::channel(10);
        let (event_sender, mut event_reciever) = mpsc::channel(10);
        let (_, exit_reciever) = mpsc::channel(10);
        let (_, file_reciever) = mpsc::channel(10);

        task::spawn(async {
            event_handler(socket_reciever, exit_reciever, file_reciever, event_sender).await;
        });

        socket_sender
            .send((String::from("live"), String::from("kaicenat")))
            .await
            .unwrap();

        assert_eq!(
            Some((String::from("kaicenat"), 1080)),
            event_reciever.recv().await
        );
    }

    #[tokio::test]
    async fn handle_socket_event() {
        let (socket_sender, socket_reciever) = mpsc::channel(10);
        let (event_sender, mut event_reciever) = mpsc::channel(10);
        let (_, exit_reciever) = mpsc::channel(10);
        let (file_sender, file_reciever) = mpsc::channel(10);

        let streamer = StreamConfig {
            name: String::from("kaicenat"),
            id: 641972806,
            quality_overides: vec![(String::from("normal"), 480)],
            streams_to_close_on: Vec::new(),
            streams_to_open_on: Vec::new(),
        };

        task::spawn(async {
            event_handler(socket_reciever, exit_reciever, file_reciever, event_sender).await;
        });

        file_sender.send(streamer).await.unwrap();

        socket_sender
            .send((String::from("live"), String::from("kaicenat")))
            .await
            .unwrap();

        assert_eq!(
            Some((String::from("kaicenat"), 480)),
            event_reciever.recv().await
        );
    }

    #[tokio::test]
    async fn handle_exit_event() {
        let (_, socket_reciever) = mpsc::channel(10);
        let (event_sender, mut event_reciever) = mpsc::channel(10);
        let (exit_sender, exit_reciever) = mpsc::channel(10);
        let (file_sender, file_reciever) = mpsc::channel(10);

        let streamer = StreamConfig {
            name: String::from("kaicenat"),
            id: 641972806,
            quality_overides: vec![(String::from("normal"), 480)],
            streams_to_close_on: Vec::new(),
            streams_to_open_on: Vec::new(),
        };

        task::spawn(async {
            event_handler(socket_reciever, exit_reciever, file_reciever, event_sender).await;
        });

        file_sender.send(streamer).await.unwrap();
        exit_sender
            .send((String::from("live"), String::from("kaicenat")))
            .await
            .unwrap();

        assert_eq!(
            Some((String::from("kaicenat"), 480)),
            event_reciever.recv().await
        );
    }

    #[tokio::test]
    async fn handle_no_override() {
        let (socket_sender, socket_reciever) = mpsc::channel(10);
        let (event_sender, mut event_reciever) = mpsc::channel(10);
        let (_, exit_reciever) = mpsc::channel(10);
        let (file_sender, file_reciever) = mpsc::channel(10);

        let streamer = StreamConfig {
            name: String::from("kaicenat"),
            id: 641972806,
            quality_overides: Vec::new(),
            streams_to_close_on: Vec::new(),
            streams_to_open_on: Vec::new(),
        };

        task::spawn(async {
            event_handler(socket_reciever, exit_reciever, file_reciever, event_sender).await;
        });

        file_sender.send(streamer).await.unwrap();

        socket_sender
            .send((String::from("live"), String::from("kaicenat")))
            .await
            .unwrap();

        assert_eq!(
            Some((String::from("kaicenat"), 1080)),
            event_reciever.recv().await
        );
    }
}
