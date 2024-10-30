use tokio::sync::mpsc::{Receiver, Sender};

pub async fn twitch_websocket(
    mut twitch_socket_file_watcher_reciever: Receiver<u32>,
    twitch_websocket_event_handler_sender: Sender<(String, String)>,
) {
}
#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn recieve_and_send() {
        use super::*;
        use tokio::sync::mpsc;
        use tokio::task;

        let (id_sender, id_reciever) = mpsc::channel(10);
        let (socket_sender, mut socket_reciever) = mpsc::channel(10);

        task::spawn(async { twitch_websocket(id_reciever, socket_sender).await });
        id_sender.send(641972806).await.unwrap();
        assert_eq!(
            socket_reciever.recv().await,
            Some((String::from("live"), String::from("kaicenat")))
        );
    }

    #[tokio::test]
    async fn recieve_and_send_multuple() {
        use super::*;
        use tokio::sync::mpsc;
        use tokio::task;

        let (id_sender, id_reciever) = mpsc::channel(10);
        let (socket_sender, mut socket_reciever) = mpsc::channel(10);

        task::spawn(async { twitch_websocket(id_reciever, socket_sender).await });
        id_sender.send(641972806).await.unwrap();
        id_sender.send(641972806).await.unwrap();
        assert_eq!(
            socket_reciever.recv().await,
            Some((String::from("live"), String::from("kaicenat")))
        );
        assert_eq!(
            socket_reciever.recv().await,
            Some((String::from("live"), String::from("kaicenat")))
        );
        id_sender.send(411377640).await.unwrap();
        assert_eq!(
            socket_reciever.recv().await,
            Some((String::from("live"), String::from("jynxzi")))
        );
        id_sender.send(207813352).await.unwrap();
        assert_eq!(
            socket_reciever.recv().await,
            Some((String::from("live"), String::from("hasanabi")))
        );
    }
}
