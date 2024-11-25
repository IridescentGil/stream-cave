use crate::UserData;
use core::panic;
use std::path::Path;

use twitch_oauth2::{
    tokens::{errors::ValidationError, UserToken},
    url, ImplicitUserTokenBuilder,
};

pub async fn create_auth_token(
    client_id: &str,
    user_access_token: &mut Option<UserToken>,
    path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()?;

    if path.join("user-data.json").exists() {
        let user_data = UserData::from_file(&path.join("user-data.json"))?;
        let token_result = UserToken::from_token(&client, user_data.access_token.into()).await;
        match token_result {
            Ok(token) => {
                *user_access_token = Some(token);
                return Ok(());
            }
            Err(error) => match error {
                ValidationError::NotAuthorized => {}
                ValidationError::Request(request_error) => {
                    // FIXME: implement waiting on request errors
                    return Err(request_error.into());
                }
                ValidationError::InvalidToken(token_error) => {
                    return Err(token_error.into());
                }
                ValidationError::RequestParseError(request_error) => {
                    return Err(request_error.into());
                }
                _ => panic!("Unkown error on parsing twitch token validation errors"),
            },
        }
    }

    let id = twitch_oauth2::ClientId::new(client_id.to_string());
    let redirect_url = url::Url::parse("https://iridescentsun.com")?;
    let mut token = ImplicitUserTokenBuilder::new(id, redirect_url).force_verify(true);

    let (url, _) = token.generate_url();
    println!("Go to this page: {}", url);

    let input = rpassword::prompt_password(
        "Paste in the resulting adress after authenticating (input hidden): ",
    )?;

    let u = url::Url::parse(&input)?;

    let map: std::collections::HashMap<_, _> = match u.fragment() {
        Some(fragment) => fragment
            .split('&')
            .map(|query| {
                let query_tuple = query.split_once('=').unwrap();
                (query_tuple.0.to_owned(), query_tuple.1.to_owned())
            })
            .collect(),
        None => u
            .query_pairs()
            .map(|cow_query| (cow_query.0.to_string(), cow_query.1.to_string()))
            .collect(),
    };

    let user_token = match (map.get("access_token"), map.get("state")) {
        (Some(access_token), Some(state)) => {
            let state_decoded = percent_encoding::percent_decode_str(state)
                .decode_utf8()
                .unwrap();
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
    *user_access_token = Some(user_token);

    Ok(())
}

#[cfg(test)]
mod tests {}
