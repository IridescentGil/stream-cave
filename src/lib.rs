//! A library to monitor and play twitch streams

mod watcher;
#[doc(inline)]
pub use crate::watcher::authentication;
#[doc(inline)]
pub use crate::watcher::authentication::create_oauth_token;
#[doc(inline)]
pub use crate::watcher::event_handler;
#[doc(inline)]
pub use crate::watcher::file_watcher;
#[doc(inline)]
pub use crate::watcher::player::get_stream;
#[doc(inline)]
pub use crate::watcher::tasks_handler;
#[doc(inline)]
pub use crate::watcher::twitch_socket;
#[doc(inline)]
pub use crate::watcher::Player;
#[doc(inline)]
pub use crate::watcher::Settings;
#[doc(inline)]
pub use crate::watcher::StreamConfig;
#[doc(inline)]
pub use crate::watcher::Streams;
#[doc(inline)]
pub use crate::watcher::UserData;
