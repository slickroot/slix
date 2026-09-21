mod api;
mod config;
mod oauth;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = config::Config::load()?;
    let _ = run(
        config,
        "https://api.x.com/2/",
        "https://api.x.com/oauth",
        &config::Config::path(),
        std::io::stdin().lock(),
        std::io::stdout(),
    )?;
    Ok(())
}

fn run(
    config: config::Config,
    api_base: &str,
    oauth_base: &str,
    path: &std::path::Path,
    input: impl std::io::BufRead,
    mut output: impl std::io::Write,
) -> Result<config::Config, Box<dyn std::error::Error>> {
    let api_base = api_base.trim_end_matches('/');
    let oauth_base = oauth_base.trim_end_matches('/');
    let mut config = config;

    let mut flow = oauth::PinFlow::for_endpoints(
        oauth_base,
        &config.consumer_key,
        &config.consumer_secret,
        reqwest::blocking::Client::new(),
    );
    writeln!(output, "{}", flow.authorize_url()?)?;
    let pin = match input.lines().next() {
        Some(pin) => pin?,
        None => {
            return Err(
                std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "no pin line").into(),
            );
        }
    };
    let (access_token, access_token_secret) = flow.exchange(&pin)?;
    config.access_token = Some(access_token);
    config.access_token_secret = Some(access_token_secret);
    config.save_to(path)?;

    let api = api::XApiClient::for_endpoint(
        api_base,
        &config.consumer_key,
        &config.consumer_secret,
        config.access_token.as_deref().unwrap(),
        config.access_token_secret.as_deref().unwrap(),
        reqwest::blocking::Client::new(),
    );
    let handle = api.users_me()?;
    writeln!(output, "{handle} · connected")?;

    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use httpmock::MockServer;

    fn test_dir(name: &str) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("slix-main-test-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn unused_config() -> config::Config {
        config::Config {
            consumer_key: "test-consumer-key".into(),
            consumer_secret: "test-consumer-secret".into(),
            access_token: None,
            access_token_secret: None,
        }
    }

    fn mock_request_token(server: &MockServer) -> httpmock::Mock<'_> {
        server.mock(|when, then| {
            when.method(httpmock::Method::POST)
                .path("/oauth/request_token");
            then.status(200)
                .body("oauth_token=RT&oauth_token_secret=RS");
        })
    }

    fn mock_access_token(server: &MockServer, status: u16) -> httpmock::Mock<'_> {
        server.mock(|when, then| {
            when.method(httpmock::Method::POST)
                .path("/oauth/access_token");
            then.status(status).body(if status == 200 {
                "oauth_token=AT&oauth_token_secret=ATS".to_string()
            } else {
                "denied".to_string()
            });
        })
    }

    fn mock_users_me<'a>(
        server: &'a MockServer,
        status: u16,
        username: &str,
    ) -> httpmock::Mock<'a> {
        server.mock(|when, then| {
            when.method(httpmock::Method::GET).path("/2/users/me");
            then.status(status).body(format!(
                r#"{{"data":{{"id":"1","name":"Slick Root","username":"{username}"}}}}"#
            ));
        })
    }

    fn api_base(server: &MockServer) -> String {
        format!("{}/2/", server.base_url())
    }

    fn oauth_base(server: &MockServer) -> String {
        format!("{}/oauth", server.base_url())
    }

    #[test]
    fn first_run_connects_prints_handle_and_persists() {
        let server = MockServer::start();
        mock_request_token(&server);
        mock_access_token(&server, 200);
        let username = "slickroot";
        mock_users_me(&server, 200, username);

        let path = test_dir("first-run").join("config.json");
        let mut output = Vec::new();

        let config = run(
            unused_config(),
            &api_base(&server),
            &oauth_base(&server),
            &path,
            std::io::Cursor::new("123456\n"),
            &mut output,
        )
        .unwrap();

        let text = String::from_utf8(output).unwrap();
        let authorize_url = format!("{}/oauth/authorize?", server.base_url());
        assert!(
            text.lines().any(|line| line.starts_with(&authorize_url)),
            "{text}"
        );
        assert_eq!(
            text.lines().last().unwrap(),
            format!("@{username} · connected")
        );

        assert!(config.is_connected());
        let persisted = config::Config::load_from(&path).unwrap();
        assert!(persisted.is_connected());
        assert_eq!(persisted, config);
    }
}
