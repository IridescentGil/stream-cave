pub mod api_structs;

use std::{
    sync::{Arc, Mutex},
    time::Duration,
};
use twitch_oauth2::UserToken;

use futures_util::StreamExt;
use tokio::{
    net::TcpStream,
    sync::mpsc::{self, Receiver, Sender},
    task::{self, yield_now},
    time::sleep,
};
use tokio_tungstenite::{
    connect_async_tls_with_config,
    tungstenite::{http::Response, Error, Message},
    MaybeTlsStream, WebSocketStream,
};

/// Create and manage twitch websocket connections and subscribe to twitch streamer live events.
///
/// Uses id's recieved from `file_watcher` to subscribe to events. When encountering a websocket
/// error or an invalid token it will send a signal through `restart_signal_sender`.
///
/// # Panics
/// Mutex lock poisoning will cause this function to panic.
///
/// # Examples
/// ```no_run
/// use tokio::sync::mpsc;
/// use std::sync::Arc;
/// use std::path::Path;
/// use stream_watcher::{Settings, twitch_socket, authentication};
///
/// #[tokio::main]
/// async fn main(){
///     const TWITCH_WEBSOCKET_URL: &str = "wss://eventsub.wss.twitch.tv/ws";
///     const TWITCH_API_URL: &str = "https://api.twitch.tv/helix/eventsub/subscriptions";
///     const CLIENT_ID: &str = "AAAAAAAAA";
///     let (restart_signal_sender, restart_signal_reciever) = mpsc::channel(1);
///     let (event_handler_sender, event_handler_reciever) = mpsc::channel(5);
///     let (file_watcher_sender, file_watcher_reciever) = mpsc::channel(5);
///
///     let mut token = None;
///     let settings = Settings::new(&Path::new("./"));
///     authentication::validate_oauth_token(&mut token, &settings.schedule).await.unwrap();
///
///     let twitch_user_access_token = Arc::new(token);
///     twitch_socket::twitch_websocket(file_watcher_reciever, event_handler_sender,
///     restart_signal_sender, TWITCH_WEBSOCKET_URL, TWITCH_API_URL, twitch_user_access_token, CLIENT_ID);
/// }
/// ```
pub fn twitch_websocket<'a: 'static>(
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
        if first_name_signal_reciever.recv().await == Some(()) {
            parse_stream_message(
                twitch_websocket_url,
                websocket_session_id,
                twitch_websocket_event_handler_sender,
                &restart_signal_sender,
            )
            .await;
        }
    });

    task::spawn(async move {
        while twitch_socket_file_watcher_reciever.is_empty() {
            yield_now().await;
        }
        if first_name_signal_sender.send(()).await == Ok(()) {
            while session_id_clone
                .lock()
                .as_ref()
                .expect("Mutex lock poisoned")
                .is_empty()
            {
                sleep(Duration::from_secs(1)).await;
            }
            while let Some(id) = twitch_socket_file_watcher_reciever.recv().await {
                subscribe_to_event(
                    &restart_signal_sender_clone,
                    twitch_api_url,
                    &twitch_user_access_token,
                    id,
                    &session_id_clone,
                    client_id,
                )
                .await;
            }
        }
    });
}

async fn parse_stream_message(
    websocket_url: &str,
    websocket_session_id: Arc<Mutex<String>>,
    twitch_websocket_event_handler_sender: Sender<(String, String)>,
    restart_signal_sender: &Sender<u8>,
) {
    let connector = tokio_tungstenite::Connector::NativeTls(
        native_tls::TlsConnector::new().expect("Unable to find TLS configuration"),
    );
    let (mut ws_stream, _) =
        match connect_async_tls_with_config(websocket_url, None, false, Some(connector)).await {
            Ok(conect) => conect,
            Err(error) => {
                eprintln!("Error: {error}, reconnecting");
                reconnect_websocket(websocket_url).await
            }
        };

    loop {
        if let Some(connection) = tokio::time::timeout(Duration::from_secs(15), ws_stream.next())
            .await
            .unwrap_or(None)
        {
            let result = parse_twitch_webocket_messages(
                connection,
                &mut ws_stream,
                &websocket_session_id,
                &twitch_websocket_event_handler_sender,
                restart_signal_sender,
            )
            .await;
            if result.is_err() {
                return;
            }
        } else {
            eprintln!("Signal timeout, attempting to reconnect");
            let _ = restart_signal_sender.send(1).await;
            return;
        }
    }
}

async fn parse_twitch_webocket_messages<'a>(
    connection: Result<Message, Error>,
    ws_stream: &'a mut WebSocketStream<MaybeTlsStream<TcpStream>>,
    websocket_session_id: &'a Arc<Mutex<String>>,
    twitch_websocket_event_handler_sender: &'a Sender<(String, String)>,
    restart_signal_sender: &'a Sender<u8>,
) -> Result<(), i8> {
    match connection {
        Ok(message) => {
            if !message.is_ping() && !message.is_pong() && !message.is_close() {
                let Ok(message) = message.to_text() else {
                    eprintln!("Error while parsing websocket message");
                    return Err(-2);
                };
                let parsed: api_structs::TwitchApi = serde_json::from_str(message)
                    .unwrap_or_else(|_| panic!("Unable to parse json response: \n{message}"));

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
            eprintln!("Error: {error}\n reconnecting");
            let _ = restart_signal_sender.send(1).await;
            return Err(-1);
        }
    }
    Ok(())
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
            parse_connection_reply_message(
                restart_signal_sender,
                reply,
                payload,
                websocket_session_id,
                ws_stream,
            )
            .await;
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
    restart_signal_sender: &'a Sender<u8>,
    reply: api_structs::ReplyMetadata,
    payload: api_structs::WebsocketPayload,
    websocket_session_id: &'a Arc<Mutex<String>>,
    ws_stream: &'a mut WebSocketStream<MaybeTlsStream<TcpStream>>,
) {
    match reply.message_type {
        api_structs::MessageType::SessionWelcome => {
            let api_structs::WebsocketPayload::Connection(welcome) = payload else {
                eprintln!("Error expecting welcome message, got:\n{payload:?}");
                return;
            };

            **websocket_session_id
                .lock()
                .as_mut()
                .expect("Mutex lock poisoned") = welcome.session.id;
            println!("successfully started websocket");
        }
        api_structs::MessageType::SessionReconnect => {
            let api_structs::WebsocketPayload::Connection(reconnect) = payload else {
                eprintln!("Error expecting reconnect message, got:\n{payload:?}");
                return;
            };

            let connector = tokio_tungstenite::Connector::NativeTls(
                native_tls::TlsConnector::new().expect("Unable to find TLS configuration"),
            );
            let connection_result = connect_async_tls_with_config(
                &reconnect
                    .session
                    .reconnect_url
                    .expect("Expected reconect url"),
                None,
                false,
                Some(connector),
            )
            .await;

            match connection_result {
                Ok(websocket) => {
                    let (new_stream, _) = websocket;
                    *ws_stream = new_stream;
                    println!("Changed connection due to reconnect request");
                }
                Err(error) => {
                    eprintln!("Error: {error},\nattempting to reconnect");
                    let _ = restart_signal_sender.send(1).await;
                }
            }
        }
        api_structs::MessageType::SessionKeepalive => {
            // Nothing to do
        }
        _ => {
            eprintln!("Error in socket message got notification in connection message");
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
                    "Error unknown reply expected subscription notification got:\n{payload:?}"
                );
                return;
            };
            if subscription.subscription.subscritpion_type == "stream.online"
                && subscription.event.event_type == api_structs::EventType::Live
            {
                let stream_name = subscription.event.broadcaster_user_login;
                twitch_websocket_event_handler_sender
                    .send((String::from("live"), stream_name))
                    .await
                    .unwrap_or_else(|error| {
                        eprintln!(
                            "Encountered error while sending notification from websocket:\n{error}"
                        );
                    });
            }
        }
        api_structs::MessageType::Revocation => {
            let api_structs::WebsocketPayload::Revocation(subscription) = payload else {
                eprintln!("Error unknown reply expected revocation notification got:\n{payload:?}");
                return;
            };
            if subscription.subscription.status
                == api_structs::SubscriptionStatus::AuthorizationRevoked
            {
                eprintln!(
                    "No user acess token or token has expired, please create new user access token"
                );
                let _ = restart_signal_sender.send(2).await;
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
            eprintln!("Error in socket message got connection message in notification");
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
    let connector = tokio_tungstenite::Connector::NativeTls(
        native_tls::TlsConnector::new().expect("Unable to find TLS configuration"),
    );
    let mut time = Duration::new(1, 0);
    let mut new_stream =
        connect_async_tls_with_config(websocket_url, None, false, Some(connector)).await;

    while new_stream.is_err() {
        let connector = tokio_tungstenite::Connector::NativeTls(
            native_tls::TlsConnector::new().expect("Unable to find TLS configuration"),
        );
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
        new_stream =
            connect_async_tls_with_config(websocket_url, None, false, Some(connector)).await;
    }
    println!("reconnected successfully");

    new_stream.expect("Expected websocket connection, but found an error")
}

async fn subscribe_to_event(
    restart_signal_sender: &Sender<u8>,
    api_url: &str,
    user_access_token: &Arc<Option<UserToken>>,
    id: u32,
    session_id: &Arc<Mutex<String>>,
    client_id: &str,
) {
    const MAX_WAIT: Duration = Duration::new(180, 0);
    let time = Duration::new(1, 0);
    let session_id = session_id
        .lock()
        .as_ref()
        .expect("Mutex lock poisoned")
        .to_string();
    let subscription = api_structs::SubscriptionBody::new_live_sub(id, session_id);

    loop {
        let subscriber = reqwest::Client::new()
            .post(api_url)
            .bearer_auth(
                (**user_access_token)
                    .as_ref()
                    .expect("Expected twitch user oauth2 token found none.")
                    .access_token
                    .as_str(),
            )
            .header("Client-Id", client_id)
            .json(&subscription)
            .send()
            .await;
        match subscriber {
            Ok(response) => {
                if response.status() == 401 {
                    let _ = restart_signal_sender.send(2).await;
                    break;
                } else if response.status() != 202 {
                    let response_status = response.status();
                    match response.text().await {
                        Ok(text) => {
                            println!("Error {response_status}: \n{text}");
                        }
                        Err(error) => {
                            eprintln!(
                        "Error {response_status}: \nEncountered error while attempting to parse response text:\n{error}"
                    );
                        }
                    }
                } else {
                    match response.text().await {
                        Ok(text) => println!("Subscribed to event:\n{text}"),
                        Err(error) => eprintln!(
                            "Error while attempting to display subscription response:\n{error}"
                        ),
                    }
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

    use super::*;
    use tokio::sync::mpsc;
    use tokio::task;

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
            );
        });

        id_sender.send(30_423_375).await.unwrap();

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
}
