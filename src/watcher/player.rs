use crate::Player;
use std::process::{Command, ExitStatus};

pub fn get_player_func(player: Player) -> impl Fn(&str, u8) -> Result<ExitStatus, std::io::Error> {
    match player {
        Player::Mpv => |stream: &str, quality: u8| {
            let mut mpv: Command = Command::new("mpv");
            if quality == 0 {
                mpv.arg(stream).arg("--ytdl-format=\"bestaudio\"");
            } else {
                mpv.arg(stream)
                    .arg(format!("--ytdl-format=\"best[height<=?{}]\"", quality));
            }
            mpv.status()
        },
        Player::Streamlink => |stream: &str, quality: u8| {
            let mut streamlink: Command = Command::new("streamlink");
            if quality == 0 {
                streamlink.arg(stream).arg("audio_only");
            } else {
                streamlink.arg(stream).arg(format!("{}p", quality));
            }
            streamlink.status()
        },
    }
}
