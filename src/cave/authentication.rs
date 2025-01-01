use crate::UserData;
use core::panic;
use std::{path::Path, time::Duration};

use twitch_oauth2::{
    tokens::{errors::ValidationError, UserToken},
    url, ImplicitUserTokenBuilder,
};

/// Validate the given the token found in the directory. If the token is valid modify
/// `user_access_token`.
///
/// # Errors
/// The function can fail due to various errors most notably network errors and an invalid token.
///
/// # Panics
/// If there token authentication returns an unkown error the function will panic
///
/// # Examples
/// ```no_run
/// #[tokio::main]
/// async fn main() {
/// use stream_cave::authentication::validate_oauth_token;
/// use std::path::Path;
///
/// let mut token = None;
/// let path = Path::new("./");
///
/// validate_oauth_token(&mut token, &path).await.unwrap();
/// }
/// ```
pub async fn validate_oauth_token(
    user_access_token: &mut Option<UserToken>,
    path: &Path,
) -> Result<(), Box<dyn std::error::Error + Sync + Send>> {
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()?;

    loop {
        if path.join("user-data.json").exists() {
            let user_data = UserData::from_file(&path.join("user-data.json"))?;
            let token_result = UserToken::from_token(&client, user_data.access_token.into()).await;
            match token_result {
                Ok(token) => {
                    *user_access_token = Some(token);
                    return Ok(());
                }
                Err(error) => {
                    match error {
                        ValidationError::NotAuthorized => {
                            eprintln!("Token not authorized please create new token, trying again in 60 seconds.");
                            tokio::time::sleep(Duration::from_secs(60)).await;
                        }
                        ValidationError::Request(_) => {
                            eprintln!("Request error when authenticating token, trying again in 60 seconds.");
                            tokio::time::sleep(Duration::from_secs(60)).await;
                        }
                        ValidationError::InvalidToken(token_error) => {
                            return Err(token_error.into());
                        }
                        ValidationError::RequestParseError(request_error) => {
                            return Err(request_error.into());
                        }
                        _ => panic!("Unkown error on parsing twitch token validation errors"),
                    }
                }
            }
        } else {
            eprintln!("No existing token, Please create token. Rechecking in 60 seconds.");
            tokio::time::sleep(Duration::from_secs(60)).await;
        }
    }
}

/// Create a twitch oauth2 token using implicit grant flow.
///
/// # Errors
/// The function can return error due to an invalid url, a failure in token creation and a failure
/// in token validation.
///
/// # Panics
/// Panics can hapen when the entered url does not have the proper query url structure
///
///# Examples
///```no_run
/// #[tokio::main]
/// async fn main() {
/// use std::path::Path;
/// use stream_cave::create_oauth_token;
///
/// let client_id = "someclientid";
/// let path = Path::new("./");
///
/// create_oauth_token(client_id, &path).await.unwrap();
/// }
///```
pub async fn create_oauth_token(
    client_id: &str,
    path: &Path,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()?;

    let id = twitch_oauth2::ClientId::new(client_id.to_string());
    let redirect_url = url::Url::parse("https://iridescentsun.com")?;
    let mut token = ImplicitUserTokenBuilder::new(id, redirect_url).force_verify(true);

    let (url, _) = token.generate_url();
    println!("Go to this page: {url}");

    let input = rpassword::prompt_password(
        "Paste in the resulting adress after authenticating (input hidden): ",
    )?;

    let input_url = url::Url::parse(&input)?;

    let map: std::collections::HashMap<_, _> = input_url.fragment().map_or_else(
        || {
            input_url
                .query_pairs()
                .map(|cow_query| (cow_query.0.to_string(), cow_query.1.to_string()))
                .collect()
        },
        |fragment| {
            fragment
                .split('&')
                .map(|query| {
                    let query_tuple = query.split_once('=').expect("Malformed url");
                    (query_tuple.0.to_owned(), query_tuple.1.to_owned())
                })
                .collect()
        },
    );

    let user_token = match (map.get("access_token"), map.get("state")) {
        (Some(access_token), Some(state)) => {
            let state_decoded = percent_encoding::percent_decode_str(state).decode_utf8()?;
            token
                .get_user_token(
                    &client,
                    Some(&state_decoded),
                    Some(access_token),
                    None,
                    None,
                )
                .await?
        }
        _ => match (map.get("error"), map.get("error_description")) {
            (Some(error), Some(error_description)) => {
                token
                    .get_user_token(&client, None, None, Some(error), Some(error_description))
                    .await?
            }
            _ => panic!("invalid url passed"),
        },
    };

    UserData::from_token(&user_token).save(&path.join("user-data.json"))?;

    Ok(())
}
