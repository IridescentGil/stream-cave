use serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug)]
pub struct TwitchApi {
    pub metadata: WebsocketMetadata,
    pub payload: WebsocketPayload,
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
pub enum WebsocketMetadata {
    Notification(NotificationMetadata),
    Reply(ReplyMetadata),
}

#[derive(Deserialize, Debug)]
pub struct NotificationMetadata {
    pub message_id: String,
    pub message_type: MessageType,
    pub message_timestamp: String,
    pub subscription_type: String,
    pub subscription_version: String,
}

#[derive(Deserialize, Debug)]
pub struct ReplyMetadata {
    pub message_id: String,
    pub message_type: MessageType,
    pub message_timestamp: String,
}

#[derive(Deserialize, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum MessageType {
    SessionWelcome,
    SessionKeepalive,
    Notification,
    SessionReconnect,
    Revocation,
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
pub enum WebsocketPayload {
    Notification(NotificationPayload),
    Connection(ConnectionPayload),
    Revocation(RevocationPayload),
    KeepAlive {},
}

#[derive(Deserialize, Debug)]
pub struct NotificationPayload {
    pub subscription: WebsocketSubscription,
    pub event: WebsocketEvent,
}

#[derive(Deserialize, Debug)]
pub struct ConnectionPayload {
    pub session: WebsocketSession,
}

#[derive(Deserialize, Debug)]
pub struct RevocationPayload {
    pub subscription: WebsocketSubscription,
}

#[derive(Deserialize, Debug)]
pub struct WebsocketSession {
    pub id: String,
    pub status: WebsocketConnectionStatus,
    pub connected_at: String,
    pub keepalive_timeout_seconds: Option<u32>,
    pub reconnect_url: Option<String>,
}

#[derive(Deserialize, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum WebsocketConnectionStatus {
    Connected,
    Reconnecting,
}

#[derive(Deserialize, Debug)]
pub struct WebsocketSubscription {
    pub id: String,
    #[serde(rename = "type")]
    pub subscritpion_type: String,
    pub version: String,
    pub status: SubscriptionStatus,
    pub cost: u32,
    pub condition: Condition,
    pub transport: Transport,
    pub created_at: String,
}

#[derive(Deserialize, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SubscriptionStatus {
    Enabled,
    AuthorizationRevoked,
    UserRemoved,
    VersionRemoved,
}

#[derive(Deserialize, Debug)]
pub struct Condition {
    pub broadcaster_user_id: String,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Transport {
    pub method: String,
    pub session_id: String,
}

#[derive(Deserialize, Debug)]
pub struct WebsocketEvent {
    pub id: String,
    pub broadcaster_user_id: String,
    pub broadcaster_user_login: String,
    pub broadcaster_user_name: String,
    #[serde(rename = "type")]
    pub event_type: EventType,
    pub started_at: String,
}

#[derive(Deserialize, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    Live,
    Playlist,
    WatchParty,
    Premiere,
    Rerun,
}

#[derive(Serialize)]
pub struct SubscriptionBody {
    #[serde(rename = "type")]
    subscritpion_type: String,
    version: String,
    condition: ConditionSubscriptionBody,
    transport: Transport,
}

impl SubscriptionBody {
    pub fn new_live_sub(id: u32, session_id: String) -> Self {
        SubscriptionBody {
            subscritpion_type: String::from("stream.live"),
            version: String::from("1"),
            condition: ConditionSubscriptionBody {
                user_id: id.to_string(),
            },
            transport: Transport {
                method: String::from("websocket"),
                session_id,
            },
        }
    }
}

#[derive(Serialize, Debug)]
pub struct ConditionSubscriptionBody {
    pub user_id: String,
}
