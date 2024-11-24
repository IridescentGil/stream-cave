pub mod api_structs;

use std::{sync::Arc, time::Duration};
use twitch_oauth2::UserToken;

use futures_util::StreamExt;
use tokio::{
    net::TcpStream,
    sync::{
        mpsc::{self, Receiver, Sender},
        Mutex,
    },
    task::{self, yield_now},
    time::sleep,
};
use tokio_tungstenite::{
    connect_async,
    tungstenite::{http::Response, Error, Message},
    MaybeTlsStream, WebSocketStream,
};

pub async fn twitch_websocket<'a: 'static>(
    mut twitch_socket_file_watcher_reciever: Receiver<u32>,
    twitch_websocket_event_handler_sender: Sender<(String, String)>,
    restart_signal_sender: Sender<u8>,
    twitch_websocket_url: &'a str,
    twitch_api_url: &'a str,
    twitch_user_access_token: Arc<Option<UserToken>>,
    client_id: &'a str,
) {
    let restart_signal_sender_clone = restart_signal_sender.clone();
    let (first_name_signal_sender, mut first_name_signal_reciever) = mpsc::channel(1);

    let websocket_session_id = Arc::new(Mutex::new(String::new()));
    let session_id_clone = websocket_session_id.clone();

    task::spawn(async move {
        first_name_signal_reciever.recv().await.unwrap();

        parse_stream_message(
            twitch_websocket_url,
            websocket_session_id,
            twitch_websocket_event_handler_sender,
            &restart_signal_sender,
        )
        .await;
    });

    task::spawn(async move {
        while twitch_socket_file_watcher_reciever.is_empty() {
            yield_now().await;
        }
        first_name_signal_sender.send(()).await.unwrap();

        while session_id_clone.lock().await.is_empty() {
            yield_now().await;
        }
        while let Some(id) = twitch_socket_file_watcher_reciever.recv().await {
            subscribe_to_event(
                &restart_signal_sender_clone,
                twitch_api_url,
                &twitch_user_access_token,
                id,
                session_id_clone.lock().await.clone(),
                client_id,
            )
            .await;
        }
    });
}

async fn parse_stream_message(
    websocket_url: &str,
    websocket_session_id: Arc<Mutex<String>>,
    twitch_websocket_event_handler_sender: Sender<(String, String)>,
    restart_signal_sender: &Sender<u8>,
) {
    let (mut ws_stream, _) = match connect_async(websocket_url).await {
        Ok(conect) => conect,
        Err(error) => {
            eprintln!("Error: {}, reconnecting", error);
            reconnect_websocket(websocket_url).await
        }
    };

    loop {
        match tokio::time::timeout(Duration::from_secs(15), ws_stream.next())
            .await
            .unwrap_or(None)
        {
            Some(connection) => {
                parse_twitch_webocket_messages(
                    connection,
                    &mut ws_stream,
                    &websocket_session_id,
                    &twitch_websocket_event_handler_sender,
                    restart_signal_sender,
                )
                .await;
            }
            None => {
                eprintln!("Connection closed");
                restart_signal_sender.send(1).await.unwrap();
            }
        }
    }
}

async fn parse_twitch_webocket_messages<'a>(
    connection: Result<Message, Error>,
    ws_stream: &'a mut WebSocketStream<MaybeTlsStream<TcpStream>>,
    websocket_session_id: &'a Arc<Mutex<String>>,
    twitch_websocket_event_handler_sender: &'a Sender<(String, String)>,
    restart_signal_sender: &'a Sender<u8>,
) {
    match connection {
        Ok(message) => {
            if !message.is_ping() && !message.is_pong() && !message.is_close() {
                let message = message.to_text().unwrap();
                let parsed: api_structs::TwitchApi = serde_json::from_str(message)
                    .unwrap_or_else(|_| panic!("Unable to parse json response: \n{}", message));

                parse_twitch_websocket_json(
                    parsed.metadata,
                    parsed.payload,
                    websocket_session_id,
                    twitch_websocket_event_handler_sender,
                    restart_signal_sender,
                    ws_stream,
                )
                .await;
            }
        }
        Err(error) => {
            eprintln!("Error: {}\n reconnecting", error);
            restart_signal_sender.send(1).await.unwrap();
        }
    }
}

async fn parse_twitch_websocket_json<'a>(
    metadata: api_structs::WebsocketMetadata,
    payload: api_structs::WebsocketPayload,
    websocket_session_id: &'a Arc<Mutex<String>>,
    twitch_websocket_event_handler_sender: &'a Sender<(String, String)>,
    restart_signal_sender: &'a Sender<u8>,
    ws_stream: &'a mut WebSocketStream<MaybeTlsStream<TcpStream>>,
) {
    match metadata {
        api_structs::WebsocketMetadata::Reply(reply) => {
            parse_connection_reply_message(reply, payload, websocket_session_id, ws_stream).await;
        }
        api_structs::WebsocketMetadata::Notification(notification) => {
            parse_connection_notification_message(
                notification,
                payload,
                twitch_websocket_event_handler_sender,
                restart_signal_sender,
            )
            .await;
        }
    }
}

async fn parse_connection_reply_message<'a>(
    reply: api_structs::ReplyMetadata,
    payload: api_structs::WebsocketPayload,
    websocket_session_id: &'a Arc<Mutex<String>>,
    ws_stream: &'a mut WebSocketStream<MaybeTlsStream<TcpStream>>,
) {
    match reply.message_type {
        api_structs::MessageType::SessionWelcome => {
            let api_structs::WebsocketPayload::Connection(welcome) = payload else {
                eprintln!("Error expecting welcome message, got:\n{:?}", payload);
                return;
            };

            *websocket_session_id.lock().await = welcome.session.id;
            println!("successfully started websocket");
        }
        api_structs::MessageType::SessionReconnect => {
            let api_structs::WebsocketPayload::Connection(reconnect) = payload else {
                eprintln!("Error expecting reconnect message, got:\n{:?}", payload);
                return;
            };

            let (new_stream, _) = connect_async(&reconnect.session.reconnect_url.unwrap())
                .await
                .unwrap();

            *ws_stream = new_stream;
            println!("Changed connection due to reconnect request");
        }
        api_structs::MessageType::SessionKeepalive => {
            // Nothing to do
        }
        _ => {
            eprintln!("Error in socket message got notification in connection message")
        }
    }
}

async fn parse_connection_notification_message(
    notification: api_structs::NotificationMetadata,
    payload: api_structs::WebsocketPayload,
    twitch_websocket_event_handler_sender: &Sender<(String, String)>,
    restart_signal_sender: &Sender<u8>,
) {
    match notification.message_type {
        api_structs::MessageType::Notification => {
            let api_structs::WebsocketPayload::Notification(subscription) = payload else {
                eprintln!(
                    "Error unknown reply expected subscription notification got:\n{:?}",
                    payload
                );
                return;
            };
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
        api_structs::MessageType::Revocation => {
            let api_structs::WebsocketPayload::Revocation(subscription) = payload else {
                eprintln!(
                    "Error unknown reply expected revocation notification got:\n{:?}",
                    payload
                );
                return;
            };
            if subscription.subscription.status
                == api_structs::SubscriptionStatus::AuthorizationRevoked
            {
                eprintln!(
                    "No user acess token or token has expired, please create new user access token"
                );
                restart_signal_sender.send(1).await.unwrap();
            } else if subscription.subscription.status
                == api_structs::SubscriptionStatus::UserRemoved
            {
                println!(
                    "Recieved revocation message as user: {} has been removed.",
                    subscription.subscription.condition.broadcaster_user_id
                );
            } else if subscription.subscription.status
                == api_structs::SubscriptionStatus::VersionRemoved
            {
                eprintln!(
                    "Recieved revocation message as subscription type \"{}\" \
                    has had the currently used version: {}, removed please check for \
                    updates to the program, and update if possible",
                    subscription.subscription.subscritpion_type, subscription.subscription.version
                );
            }
        }
        _ => {
            eprintln!("Error in socket message got connection message in notification")
        }
    }
}

async fn reconnect_websocket(
    websocket_url: &str,
) -> (
    WebSocketStream<MaybeTlsStream<TcpStream>>,
    Response<Option<Vec<u8>>>,
) {
    const MAX_WAIT: Duration = Duration::new(180, 0);
    let mut time = Duration::new(1, 0);
    let mut new_stream = connect_async(websocket_url).await;

    while new_stream.is_err() {
        time = match time.cmp(&MAX_WAIT) {
            std::cmp::Ordering::Less => time * 2,
            std::cmp::Ordering::Greater => MAX_WAIT,
            std::cmp::Ordering::Equal => time,
        };
        eprintln!(
            "Connection failed, re-attempting in {} secs.",
            time.as_secs()
        );
        sleep(time).await;
        new_stream = connect_async(websocket_url).await;
    }
    println!("reconnected successfully");

    new_stream.unwrap()
}

async fn subscribe_to_event(
    restart_signal_sender: &Sender<u8>,
    api_url: &str,
    user_access_token: &Arc<Option<UserToken>>,
    id: u32,
    session_id: String,
    client_id: &str,
) {
    const MAX_WAIT: Duration = Duration::new(180, 0);
    let time = Duration::new(1, 0);
    let subscription = api_structs::SubscriptionBody::new_live_sub(id, session_id);

    loop {
        let subscriber = reqwest::Client::new()
            .post(api_url)
            .bearer_auth((**user_access_token).clone().unwrap().access_token.as_str())
            .header("Client-Id", client_id)
            .json(&subscription)
            .send()
            .await;
        match subscriber {
            Ok(response) => {
                if response.status() == 401 {
                    restart_signal_sender.send(1).await.unwrap();
                } else if response.status() != 202 {
                    println!(
                        "Error {}: \n{}",
                        response.status(),
                        response.text().await.unwrap()
                    );
                } else {
                    println!("Subscribed to event:\n{}", response.text().await.unwrap());
                }
                break;
            }
            Err(error) => {
                match time.cmp(&MAX_WAIT) {
                    std::cmp::Ordering::Less => time * 2,
                    std::cmp::Ordering::Greater => MAX_WAIT,
                    std::cmp::Ordering::Equal => time,
                };
                eprintln!("Error: {}, retrying in {} secs", error, time.as_secs());
                sleep(time).await;
            }
        }
    }
}
#[cfg(test)]
mod tests {

    use std::str::FromStr;

    use tokio::{process, time::timeout};
    use twitch_oauth2::AccessToken;
    #[tokio::test]
    async fn recieve_and_send() {
        const TWITCH_WEBSOCKET_URL: &str = "ws://127.0.0.1:3200/ws";
        const TWITCH_API_URL: &str = "http://localhost:3200/eventsub/subscriptions";

        let user_access_token = Some(UserToken::from_existing_unchecked(
            AccessToken::from_str("").unwrap(),
            None,
            "",
            None,
            "".into(),
            "".into(),
            Some(vec![]),
            Some(Duration::from_secs(10000)),
        ));
        let twitch_user_access_token = Arc::new(user_access_token);

        use super::*;
        use tokio::sync::mpsc;
        use tokio::task;

        let (id_sender, id_reciever) = mpsc::channel(10);
        let (socket_sender, mut socket_reciever) = mpsc::channel(10);
        let (restart_signal_sender, _) = mpsc::channel(1);

        let mut child = process::Command::new("twitch-cli")
            .args(["event", "websocket", "start-server", "-S", "-p", "3200"])
            .kill_on_drop(true)
            .spawn()
            .unwrap();

        task::spawn(async {
            twitch_websocket(
                id_reciever,
                socket_sender,
                restart_signal_sender,
                TWITCH_WEBSOCKET_URL,
                TWITCH_API_URL,
                twitch_user_access_token,
                "AAAA",
            )
            .await
        });

        id_sender.send(30423375).await.unwrap();

        sleep(Duration::from_secs(5)).await;
        process::Command::new("twitch-cli")
            .args(["event", "trigger", "stream.online", "--transport=websocket"])
            .output()
            .await
            .unwrap();
        assert_eq!(
            timeout(Duration::from_secs(15), socket_reciever.recv())
                .await
                .unwrap(),
            Some((String::from("live"), String::from("testBroadcaster")))
        );
        child.kill().await.unwrap();
        child.wait().await.unwrap();
    }

    #[ignore = "Test needs to be fixed, unknown if twitch cli can subscribe multiple times"]
    #[tokio::test]
    async fn recieve_and_send_multuple() {
        const TWITCH_WEBSOCKET_URL: &str = "ws://127.0.0.1:8080/ws";
        const TWITCH_API_URL: &str = "http://localhost:8080/eventsub/subscriptions";
        let twitch_user_access_token = Arc::new(None);
        let (restart_signal_sender, _) = mpsc::channel(1);

        use super::*;
        use tokio::sync::mpsc;
        use tokio::task;

        let (id_sender, id_reciever) = mpsc::channel(10);
        let (socket_sender, mut socket_reciever) = mpsc::channel(10);

        task::spawn(async {
            twitch_websocket(
                id_reciever,
                socket_sender,
                restart_signal_sender,
                TWITCH_WEBSOCKET_URL,
                TWITCH_API_URL,
                twitch_user_access_token,
                "AAAA",
            )
            .await
        });
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
