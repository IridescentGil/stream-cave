use crate::watcher::twitch_socket::api_structs;
use std::{process::ExitStatus, sync::Arc, time::Duration};
use twitch_oauth2::UserToken;

use crate::{watcher::player, Player};
use tokio::{
    sync::mpsc::{Receiver, Sender},
    task,
    time::sleep,
};

pub async fn task_spawner(
    mut task_spawner_event_handler_reciever: Receiver<(String, u16)>,
    task_spawner_exit_handler_sender: Sender<(
        String,
        Result<std::process::ExitStatus, std::io::Error>,
    )>,
    player: Player,
    website: String,
) {
    while let Some((streamer_name, quality)) = task_spawner_event_handler_reciever.recv().await {
        let stream = format!("{}{}", website, streamer_name.clone());
        let player_func = player::get_stream(player, stream, quality).await;
        let sender_clone = task_spawner_exit_handler_sender.clone();
        task::spawn(async move {
            sender_clone
                .send((streamer_name, player_func.await))
                .await
                .unwrap()
        });
    }
}
pub async fn exit_handler(
    mut exit_handler_task_spawner_reciever: Receiver<(
        String,
        Result<std::process::ExitStatus, std::io::Error>,
    )>,
    exit_handler_event_handler_sender: Sender<(String, String)>,
    restart_signal_sender: Sender<u8>,
    api_url: String,
    user_access_token: Arc<Option<UserToken>>,
    client_id: &str,
) {
    while let Some((stream_name, result)) = exit_handler_task_spawner_reciever.recv().await {
        match result {
            Ok(exit_status) => {
                handle_exit_status(
                    stream_name,
                    exit_status,
                    &exit_handler_event_handler_sender,
                    &restart_signal_sender,
                    &api_url,
                    &user_access_token,
                    client_id,
                )
                .await;
            }
            Err(error) => {
                eprintln!("Error starting stream: {}", error);
            }
        }
    }
}

async fn handle_exit_status<'a>(
    stream_name: String,
    exit_status: ExitStatus,
    exit_handler_event_handler_sender: &'a Sender<(String, String)>,
    restart_signal_sender: &'a Sender<u8>,
    api_url: &'a String,
    user_access_token: &'a Arc<Option<UserToken>>,
    client_id: &'a str,
) {
    if !exit_status.success() {
        let mut wait_time = Duration::from_secs(1);
        const MAX_WAIT_TIME: Duration = Duration::from_secs(180);

        loop {
            let request = reqwest::Client::new()
                .get(api_url)
                .query(&[("query", &stream_name)])
                .bearer_auth(
                    (**user_access_token)
                        .as_ref()
                        .unwrap()
                        .access_token
                        .as_str(),
                )
                .header("Client-Id", client_id)
                .send()
                .await;

            match request {
                Ok(response) => {
                    if response.status() == 200 {
                        let search_results = response.json::<api_structs::StreamSearch>().await;
                        if search_results.is_err() {
                            eprintln!("Error malformed response data recieved when checking stream status after bad exit");
                            return;
                        }
                        let json_data = search_results.unwrap();
                        let is_live = json_data
                            .data
                            .iter()
                            .find(|data_set| data_set.broadcaster_login == stream_name)
                            .unwrap()
                            .is_live;
                        if is_live {
                            exit_handler_event_handler_sender
                                .send((String::from("retry"), stream_name))
                                .await
                                .unwrap();
                            return;
                        }
                    } else if response.status() == 401 {
                        let _ = restart_signal_sender.send(2).await;
                        return;
                    } else {
                        eprintln!("Unexpected response:{}", response.text().await.unwrap());
                        return;
                    }
                }
                Err(error) => {
                    wait_time = match wait_time.cmp(&MAX_WAIT_TIME) {
                        std::cmp::Ordering::Less => wait_time * 2,
                        std::cmp::Ordering::Equal => wait_time,
                        std::cmp::Ordering::Greater => MAX_WAIT_TIME,
                    };

                    eprintln!(
                        "Error {}\n re-attempting api call for {}'s stream status in {} secs",
                        error,
                        stream_name,
                        wait_time.as_secs()
                    );
                    sleep(wait_time).await;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{str::FromStr, sync::Arc, time::Duration};

    use super::*;
    use tokio::{
        process::Command,
        sync::mpsc,
        task,
        time::{sleep, timeout},
    };
    use twitch_oauth2::AccessToken;

    async fn create_key(port: u16) -> (Option<UserToken>, String) {
        let clients = reqwest::Client::new()
            .get(format!("http://localhost:{}/units/clients", port))
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap();
        let id = clients.find("ID").unwrap();
        let secret = clients.find("Secret").unwrap();
        let name = clients.find("Name").unwrap();

        let client_id: String = clients
            .chars()
            .skip(id + 5)
            .take((secret - 3) - (id + 5))
            .collect();

        let secret: String = clients
            .chars()
            .skip(secret + 9)
            .take((name - 3) - (secret + 9))
            .collect();

        let auth = reqwest::Client::new()
            .post(format!("http://localhost:{}/auth/authorize", port))
            .query(&[
                ("client_id", &client_id),
                ("client_secret", &secret),
                ("grant_type", &"user_token".to_string()),
                ("user_id", &"96359538".to_string()),
                ("scope", &"".to_string()),
            ])
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap();

        let token = auth.find("access_token").unwrap() + 15;
        let refresh = auth.find("refresh").unwrap() - 3;

        let user_access_token: String = auth.chars().skip(token).take(refresh - token).collect();
        let user_access_token = Some(UserToken::from_existing_unchecked(
            AccessToken::from_str(&user_access_token).unwrap(),
            None,
            client_id.clone(),
            None,
            "".into(),
            "".into(),
            Some(vec![]),
            Some(Duration::from_secs(10000)),
        ));

        (user_access_token, client_id)
    }

    #[tokio::test]
    async fn spawn_task() {
        const FILE: &str = "tests/resources/";

        let (exit_sender, mut exit_reciever) = mpsc::channel(10);
        let (event_sender, event_reciever) = mpsc::channel(10);
        let exit_status = Command::new("ls").status().await;

        let fake_streamer_name = String::from("video.mkv");
        event_sender
            .send((fake_streamer_name.clone(), 1080))
            .await
            .unwrap();

        drop(event_sender);
        task_spawner(event_reciever, exit_sender, Player::Mpv, FILE.to_string()).await;

        let (result_name, result_status) = timeout(Duration::from_secs(15), exit_reciever.recv())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(fake_streamer_name, result_name);
        assert_eq!(exit_status.unwrap(), result_status.unwrap());
    }

    #[tokio::test]
    async fn handle_good_exit() {
        const PORT: u16 = 5421;
        let api_url = format!("http://localhost:{}/mock/search/channels", PORT);
        let (process_sender, process_reciever) = mpsc::channel(10);
        let (exit_sender, exit_reciever) = mpsc::channel(10);
        let (restart_signal_sender, _) = mpsc::channel(1);

        let mut child = Command::new("twitch-cli")
            .args(["mock-api", "start", "-p", &PORT.to_string()])
            .kill_on_drop(true)
            .spawn()
            .unwrap();

        task::spawn(async {
            sleep(Duration::from_secs(2)).await;
            let (user_access_token, client_id) = create_key(PORT).await;
            let user_access_token = Arc::new(user_access_token);
            exit_handler(
                process_reciever,
                exit_sender,
                restart_signal_sender,
                api_url,
                user_access_token,
                &client_id,
            )
            .await;
        });

        process_sender
            .send((
                String::from("fishermarston19"),
                Command::new("ls").status().await,
            ))
            .await
            .unwrap();

        assert!(exit_reciever.is_empty());
        child.kill().await.unwrap();
        child.wait().await.unwrap();
    }

    #[tokio::test]
    async fn handle_bad_exit() {
        const PORT: u16 = 5422;
        let api_url = format!("http://localhost:{}/mock/search/channels", PORT);
        let (process_sender, process_reciever) = mpsc::channel(10);
        let (exit_sender, mut exit_reciever) = mpsc::channel(10);
        let (restart_signal_sender, _) = mpsc::channel(1);

        let mut child = Command::new("twitch-cli")
            .args(["mock-api", "start", "-p", &PORT.to_string()])
            .kill_on_drop(true)
            .spawn()
            .unwrap();

        task::spawn(async {
            sleep(Duration::from_secs(2)).await;
            let (user_access_token, client_id) = create_key(PORT).await;
            let user_access_token = Arc::new(user_access_token);
            exit_handler(
                process_reciever,
                exit_sender,
                restart_signal_sender,
                api_url,
                user_access_token,
                &client_id,
            )
            .await;
        });

        process_sender
            .send((
                String::from("fishermarston19"),
                Command::new("ls").arg("nonexistent").status().await,
            ))
            .await
            .unwrap();

        assert_eq!(
            timeout(Duration::from_secs(15), exit_reciever.recv())
                .await
                .unwrap(),
            Some((String::from("retry"), String::from("fishermarston19")))
        );
        child.kill().await.unwrap();
        child.wait().await.unwrap();
    }

    #[tokio::test]
    async fn handle_bad_exit_stream_ended() {
        const PORT: u16 = 8502;
        let api_url = format!("http://localhost:{}/mock/search/channels", PORT);
        let (process_sender, process_reciever) = mpsc::channel(10);
        let (exit_sender, exit_reciever) = mpsc::channel(10);
        let (restart_signal_sender, _) = mpsc::channel(1);

        let mut child = Command::new("twitch-cli")
            .args(["mock-api", "start", "-p", &PORT.to_string()])
            .kill_on_drop(true)
            .spawn()
            .unwrap();

        task::spawn(async {
            sleep(Duration::from_secs(2)).await;
            let (user_access_token, client_id) = create_key(PORT).await;
            let user_access_token = Arc::new(user_access_token);
            exit_handler(
                process_reciever,
                exit_sender,
                restart_signal_sender,
                api_url,
                user_access_token,
                &client_id,
            )
            .await;
        });

        process_sender
            .send((
                String::from("gordongordon358"),
                Command::new("ls").arg("nonexistent").status().await,
            ))
            .await
            .unwrap();

        assert!(exit_reciever.is_empty());
        child.kill().await.unwrap();
        child.wait().await.unwrap();
    }

    #[tokio::test]
    async fn handle_no_internet_exit() {
        const PORT: u16 = 8423;
        let api_url = format!("http://localhost:{}/mock/search/channels", PORT);
        let (process_sender, process_reciever) = mpsc::channel(10);
        let (exit_sender, exit_reciever) = mpsc::channel(10);
        let (restart_signal_sender, _) = mpsc::channel(1);

        let mut child = Command::new("twitch-cli")
            .args(["mock-api", "start", "-p", &PORT.to_string()])
            .kill_on_drop(true)
            .spawn()
            .unwrap();

        // FIX the next part can happen before the server is fully up,
        // causing the test to fail.
        sleep(Duration::from_secs(3)).await;

        let (user_access_token, client_id) = create_key(PORT).await;
        let user_access_token = Arc::new(user_access_token);

        child.kill().await.unwrap();
        child.wait().await.unwrap();

        process_sender
            .send((
                String::from("fishermarston19"),
                Command::new("ls").arg("nonexistent").status().await,
            ))
            .await
            .unwrap();

        tokio::time::timeout(
            Duration::from_secs(5),
            exit_handler(
                process_reciever,
                exit_sender,
                restart_signal_sender,
                api_url,
                user_access_token,
                &client_id,
            ),
        )
        .await
        .unwrap_err();

        assert!(exit_reciever.is_empty());
    }
}
