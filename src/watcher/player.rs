use crate::Player;
use std::{
    future::Future,
    process::{Command, ExitStatus},
};

pub async fn get_stream<'a, 'b>(
    player: &'a Player,
    stream: &'a str,
    quality: u16,
) -> impl Future<Output = Result<ExitStatus, std::io::Error>> + 'a {
    async move {
        match player {
            Player::Mpv => {
                let mut mpv: Command = Command::new("mpv");
                if quality == 0 {
                    mpv.arg(stream)
                        .arg("--no-resume-playback")
                        .arg("--ytdl-format=bestaudio");
                } else {
                    mpv.arg(stream)
                        .arg("--no-resume-playback")
                        .arg(format!("--ytdl-format=best[height<=?{}]", quality));
                }
                println!("mpv: {:?}", mpv);
                mpv.status()
            }
            Player::Streamlink => {
                let mut streamlink: Command = Command::new("streamlink");
                if quality == 0 {
                    streamlink.arg(stream).arg("audio_only");
                } else {
                    streamlink.arg(stream).arg(format!("{}p", quality));
                }
                streamlink.status()
            }
        }
    }
}
