use crate::Player;
use std::{future::Future, process::ExitStatus};
use tokio::process::Command;

/// Play the given stream and return a future with the exit status.
///
/// # Examples
/// ```no_run
/// use stream_cave::get_stream;
/// use stream_cave::Player;
///
/// #[tokio::main]
/// async fn main(){
///     let stream = String::from("https://twitch.tv/jynxzi");
///     let quality = 720;
///
///     let play = get_stream(Player::Mpv, stream, quality).await;
/// }
/// ```
///
///
pub async fn get_stream<'a, 'b>(
    player: Player,
    stream: String,
    quality: u16,
) -> impl Future<Output = Result<ExitStatus, std::io::Error>> + 'a {
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
                    .arg(format!("--ytdl-format=best[height<=?{quality}]"));
            }
            mpv.status()
        }
        Player::Streamlink => {
            let mut streamlink: Command = Command::new("streamlink");
            if quality == 0 {
                streamlink.arg(stream).arg("audio_only");
            } else {
                streamlink.arg(stream).arg(format!("{quality}p"));
            }
            streamlink.status()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn play_mpv() {
        let video = String::from("file://./tests/resources/video.mkv");
        let mpv = get_stream(Player::Mpv, video, 1080).await;

        let exit_code = mpv.await.unwrap();

        assert!(exit_code.success());
    }
}
