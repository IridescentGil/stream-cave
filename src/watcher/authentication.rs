use twitch_oauth2::{tokens::UserToken, url, ImplicitUserTokenBuilder};

pub async fn create_auth_token(
    client_id: &str,
    user_access_token: &mut Option<UserToken>,
) -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()?;

    let id = twitch_oauth2::ClientId::new(client_id.to_string());
    let redirect_url = url::Url::parse("https://iridescentsun.com")?;
    let mut token = ImplicitUserTokenBuilder::new(id, redirect_url).force_verify(true);

    let (url, _) = token.generate_url();
    println!("Go to this page: {}", url);

    let input = rpassword::prompt_password(
        "Paste in the resulting adress after authenticating (input hidden): ",
    )?;

    let u = url::Url::parse(&input)?;
    // FIXME: clean this up
    let map: std::collections::HashMap<_, _> = u
        .fragment()
        .unwrap()
        .split('&')
        .map(|query| {
            let mut ww = query.split('=');
            (ww.next().unwrap().to_owned(), ww.next().unwrap().to_owned())
        })
        .collect();

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

    *user_access_token = Some(user_token);

    Ok(())
}

#[cfg(test)]
mod tests {}
