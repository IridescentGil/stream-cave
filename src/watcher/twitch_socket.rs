mod api_structs;

use std::sync::Arc;

use futures_util::StreamExt;
use tokio::{
    sync::{
        mpsc::{Receiver, Sender},
        Mutex,
    },
    task::{self, yield_now},
};
use tokio_tungstenite::{connect_async, tungstenite::Message};

pub async fn twitch_websocket(
    mut twitch_socket_file_watcher_reciever: Receiver<u32>,
    twitch_websocket_event_handler_sender: Sender<(String, String)>,
) {
    //FIXME: Change to real url
    const TWITCH_WEBSOCKET_URL: &str = "ws://127.0.0.1:8080/ws";
    let stream_name_recieved = Arc::new(Mutex::new(true));
    let stream_name_clone = stream_name_recieved.clone();

    let websocket_session_id = Arc::new(Mutex::new(String::new()));
    let session_id_clone = websocket_session_id.clone();

    task::spawn(async move {
        while *stream_name_clone.lock().await {
            yield_now().await;
        }

        parse_stream_message(
            TWITCH_WEBSOCKET_URL.to_string(),
            websocket_session_id,
            twitch_websocket_event_handler_sender,
        )
        .await;
    });
    task::spawn(async move {
        while *stream_name_recieved.lock().await {
            *stream_name_recieved.lock().await = twitch_socket_file_watcher_reciever.is_empty();
        }
        while let Some(id) = twitch_socket_file_watcher_reciever.recv().await {
            subscribe_to_event(id, session_id_clone.lock().await.clone()).await;
        }
    });
}

async fn parse_stream_message(
    websocket_url: String,
    websocket_session_id: Arc<Mutex<String>>,
    twitch_websocket_event_handler_sender: Sender<(String, String)>,
) {
    let (mut ws_stream, _) = connect_async(websocket_url).await.unwrap();

    while let Some(Ok(message)) = ws_stream.next().await {
        if !message.is_ping() && !message.is_pong() && !message.is_close() {
            let message = message.to_text().unwrap();
            let parsed: api_structs::TwitchApi = serde_json::from_str(message)
                .unwrap_or_else(|_| panic!("Unable to parse json response: \n{}", message));

            match parsed.metadata {
                api_structs::WebsocketMetadata::Reply(reply) => {
                    match reply.message_type {
                        api_structs::MessageType::SessionWelcome => {
                            if let api_structs::WebsocketPayload::Connection(welcome) =
                                parsed.payload
                            {
                                *websocket_session_id.lock().await = welcome.session.id;
                            }
                        }
                        api_structs::MessageType::SessionReconnect => {
                            if let api_structs::WebsocketPayload::Connection(reconnect) =
                                parsed.payload
                            {
                                let (new_stream, _) =
                                    connect_async(&reconnect.session.reconnect_url.unwrap())
                                        .await
                                        .unwrap();
                                ws_stream = new_stream;
                            }
                        }
                        api_structs::MessageType::SessionKeepalive => {
                            // FIXME: implement keepalive or event notificatopn checking probably ising
                            // duration in instant::now
                        }
                        _ => {
                            eprintln!(
                                "Error in socket message got notification in connection message"
                            )
                        }
                    }
                }
                api_structs::WebsocketMetadata::Notification(notification) => {
                    match notification.message_type {
                        api_structs::MessageType::Notification => {
                            if let api_structs::WebsocketPayload::Notification(subscription) =
                                parsed.payload
                            {
                                if subscription.subscription.subscritpion_type == "stream.online"
                                    && subscription.event.event_type == api_structs::EventType::Live
                                {
                                    let stream_name = subscription.event.broadcaster_user_name;
                                    twitch_websocket_event_handler_sender
                                        .send((String::from("live"), stream_name))
                                        .await
                                        .unwrap();
                                }
                            }
                        }
                        api_structs::MessageType::Revocation => {}
                        _ => {
                            eprintln!(
                                "Error in socket message got connection message in notification"
                            )
                        }
                    }
                }
            }
        }
    }
}

async fn subscribe_to_event(id: u32, session_id: String) {
    let subscription = api_structs::SubscriptionBody::new_live_sub(id, session_id);
    let subscriber = reqwest::Client::new()
        .post("")
        .bearer_auth("")
        .header("Client-Id", "uty2ua26tqh28rzn3jketggzu98t6b")
        .json(&subscription)
        .send()
        .await;
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
