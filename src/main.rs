mod accounts;
mod api;
mod config;
mod history;
mod model;
mod oauth;
mod report;
mod view;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = config::Config::load()?;
    let api_base = "https://api.x.com/2/";
    let oauth_base = "https://api.x.com/oauth";
    let path = config::Config::path();
    let history = history::History::new(config.data_dir.join("history"));
    let mut output = std::io::stdout();

    match connect(
        config,
        api_base,
        oauth_base,
        &path,
        std::io::stdin().lock(),
        &mut output,
    ) {
        Ok((_config, api, me)) => {
            let now = chrono::Local::now();
            let latest = api.latest_replies(&me.id)?;
            save_replies(&history, latest)?;
            write_report(&me, &history, now, &mut output)?;
        }
        Err(err) => match err.downcast::<Reconnected>() {
            Ok(_) => {}
            Err(err) => return Err(err),
        },
    }
    Ok(())
}

fn write_report(
    me: &model::Me,
    history: &history::History,
    now: chrono::DateTime<chrono::Local>,
    output: &mut impl std::io::Write,
) -> Result<(), Box<dyn std::error::Error>> {
    let counts = history.last_30_day_counts(now.date_naive())?;
    let date = now.date_naive().pred_opt().unwrap();
    let today_count = history.load(now.date_naive())?.unwrap_or_default().len() as u64;
    let yesterday = history.load(date)?.unwrap_or_default();
    let all_replies = history.load_all()?;
    let ranks = accounts::rank(&all_replies);
    let dataset = view::Dataset {
        handle: me.handle.clone(),
        counts,
        yesterday,
        ranks,
        today_count,
    };
    for (i, view) in view::all().iter().enumerate() {
        if i > 0 {
            writeln!(output)?;
            writeln!(output, "---")?;
        }
        let title = view.title();
        if !title.is_empty() {
            writeln!(output, "{title}")?;
        }
        writeln!(output, "{}", view.render(&dataset, now))?;
    }
    Ok(())
}

fn save_replies(
    history: &history::History,
    replies: Vec<model::Reply>,
) -> Result<(), history::HistoryError> {
    let mut by_day: std::collections::BTreeMap<chrono::NaiveDate, Vec<model::Reply>> =
        std::collections::BTreeMap::new();
    for reply in replies {
        let date = reply.created_at.with_timezone(&chrono::Local).date_naive();
        by_day.entry(date).or_default().push(reply);
    }
    for (date, replies) in by_day {
        history.save(date, &replies)?;
    }
    Ok(())
}

#[derive(Debug)]
struct Reconnected(#[allow(dead_code)] config::Config);

impl std::fmt::Display for Reconnected {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "reconnected without verifying the new tokens")
    }
}

impl std::error::Error for Reconnected {}

fn connect(
    config: config::Config,
    api_base: &str,
    oauth_base: &str,
    path: &std::path::Path,
    input: impl std::io::BufRead,
    mut output: impl std::io::Write,
) -> Result<(config::Config, api::XApiClient, model::Me), Box<dyn std::error::Error>> {
    let api_base = api_base.trim_end_matches('/');
    let oauth_base = oauth_base.trim_end_matches('/');
    let mut config = config;

    let reconnecting = if config.is_connected() {
        let api = api::XApiClient::for_endpoint(
            api_base,
            &config.consumer_key,
            &config.consumer_secret,
            config.access_token.as_deref().unwrap(),
            config.access_token_secret.as_deref().unwrap(),
            reqwest::blocking::Client::new(),
        );
        match api.users_me() {
            Ok(me) => return Ok((config, api, me)),
            Err(_) => {
                writeln!(output, "reconnecting…")?;
                true
            }
        }
    } else {
        false
    };

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

    if reconnecting {
        return Err(Box::new(Reconnected(config)));
    }

    let api = api::XApiClient::for_endpoint(
        api_base,
        &config.consumer_key,
        &config.consumer_secret,
        config.access_token.as_deref().unwrap(),
        config.access_token_secret.as_deref().unwrap(),
        reqwest::blocking::Client::new(),
    );
    let me = api.users_me()?;
    Ok((config, api, me))
}

#[cfg(test)]
mod tests {
    use super::*;
    use httpmock::MockServer;

    fn connect_and_write_report(
        config: config::Config,
        api_base: &str,
        oauth_base: &str,
        path: &std::path::Path,
        history: &history::History,
        input: impl std::io::BufRead,
        mut output: impl std::io::Write,
        now: chrono::DateTime<chrono::Local>,
    ) -> Result<config::Config, Box<dyn std::error::Error>> {
        match connect(config, api_base, oauth_base, path, input, &mut output) {
            Ok((config, api, me)) => {
                let latest = api.latest_replies(&me.id)?;
                save_replies(history, latest)?;
                write_report(&me, history, now, &mut output)?;
                Ok(config)
            }
            Err(err) => match err.downcast::<Reconnected>() {
                Ok(reconnected) => Ok(reconnected.0),
                Err(err) => Err(err),
            },
        }
    }

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
            data_dir: test_dir("unused-config"),
        }
    }

    fn test_history(name: &str) -> history::History {
        history::History::new(test_dir(name))
    }

    fn connected_config() -> config::Config {
        config::Config {
            consumer_key: "test-consumer-key".into(),
            consumer_secret: "test-consumer-secret".into(),
            access_token: Some("test-access-token".into()),
            access_token_secret: Some("test-access-token-secret".into()),
            data_dir: test_dir("connected-config"),
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

    fn mock_tweets<'a>(server: &'a MockServer, status: u16, body: &str) -> httpmock::Mock<'a> {
        server.mock(|when, then| {
            when.method(httpmock::Method::GET).path("/2/users/1/tweets");
            then.status(status).body(body.to_string());
        })
    }

    const NO_TWEETS: &str = r#"{"meta":{"result_count":0}}"#;

    fn tweet_json(id: &str, created_at: chrono::DateTime<chrono::Utc>, to_user_id: &str) -> String {
        format!(
            r#"{{"id":"{id}","created_at":"{}","text":"hi","public_metrics":{{"impression_count":1,"like_count":2}},"non_public_metrics":{{"user_profile_clicks":3}},"referenced_tweets":[{{"type":"replied_to","id":"100"}}],"in_reply_to_user_id":"{to_user_id}"}}"#,
            created_at.to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
        )
    }

    fn included_user_json(id: &str, username: &str) -> String {
        format!(r#"{{"id":"{id}","username":"{username}"}}"#)
    }

    fn tweets_body(tweets: &[String], users: &[String]) -> String {
        format!(
            r#"{{"data":[{}],"includes":{{"users":[{}]}}}}"#,
            tweets.join(","),
            users.join(",")
        )
    }

    fn now() -> chrono::DateTime<chrono::Local> {
        chrono::Local::now()
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
        mock_tweets(&server, 200, NO_TWEETS);

        let path = test_dir("first-run").join("config.json");
        let mut output = Vec::new();

        let config = connect_and_write_report(
            unused_config(),
            &api_base(&server),
            &oauth_base(&server),
            &path,
            &test_history("first-run"),
            std::io::Cursor::new("123456\n"),
            &mut output,
            now(),
        )
        .unwrap();

        let text = String::from_utf8(output).unwrap();
        let authorize_url = format!("{}/oauth/authorize?", server.base_url());
        assert!(
            text.lines().any(|line| line.starts_with(&authorize_url)),
            "{text}"
        );
        assert!(
            text.lines()
                .any(|line| line == format!("@{username} · connected")),
            "{text}"
        );

        assert!(config.is_connected());
        let persisted = config::Config::load_from(&path).unwrap();
        assert!(persisted.is_connected());
        assert_eq!(persisted, config);
    }

    #[test]
    fn verify_on_startup_prints_handle_without_oauth() {
        let server = MockServer::start();
        let request_token = mock_request_token(&server);
        let access_token = mock_access_token(&server, 200);
        let username = "slickroot";
        mock_users_me(&server, 200, username);
        mock_tweets(&server, 200, NO_TWEETS);

        let path = test_dir("verify-on-startup").join("config.json");
        let mut output = Vec::new();

        let config = connect_and_write_report(
            connected_config(),
            &api_base(&server),
            &oauth_base(&server),
            &path,
            &test_history("verify-on-startup"),
            std::io::Cursor::new(""),
            &mut output,
            now(),
        )
        .unwrap();

        let text = String::from_utf8(output).unwrap();
        assert!(text.contains(&format!("@{username} · connected")));
        assert_eq!(request_token.calls(), 0);
        assert_eq!(access_token.calls(), 0);
        assert!(config.is_connected());
    }

    #[test]
    fn reconnect_on_failed_verification_overwrites_tokens() {
        let server = MockServer::start();
        mock_request_token(&server);
        mock_access_token(&server, 200);
        let username = "slickroot";
        mock_users_me(&server, 401, username);
        let tweets = mock_tweets(&server, 200, NO_TWEETS);

        let path = test_dir("reconnect").join("config.json");
        let mut output = Vec::new();

        let config = connect_and_write_report(
            connected_config(),
            &api_base(&server),
            &oauth_base(&server),
            &path,
            &test_history("reconnect"),
            std::io::Cursor::new("123456\n"),
            &mut output,
            now(),
        )
        .unwrap();

        let text = String::from_utf8(output).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines[0], "reconnecting…");
        assert!(lines[1].starts_with(&format!("{}/oauth/authorize?", server.base_url())));
        assert_eq!(lines.len(), 2, "unexpected extra output: {text}");
        assert_eq!(tweets.calls(), 0);

        assert_eq!(config.access_token.as_deref(), Some("AT"));
        assert_eq!(config.access_token_secret.as_deref(), Some("ATS"));
        let persisted = config::Config::load_from(&path).unwrap();
        assert_eq!(persisted.access_token.as_deref(), Some("AT"));
        assert_eq!(persisted.access_token_secret.as_deref(), Some("ATS"));
    }

    #[test]
    fn re_open_with_persisted_tokens_stays_connected_without_oauth() {
        let server = MockServer::start();
        let request_token = mock_request_token(&server);
        let access_token = mock_access_token(&server, 200);
        let username = "slickroot";
        let users_me = mock_users_me(&server, 200, username);
        mock_tweets(&server, 200, NO_TWEETS);

        let path = test_dir("re-open").join("config.json");

        let mut first_output = Vec::new();
        connect_and_write_report(
            unused_config(),
            &api_base(&server),
            &oauth_base(&server),
            &path,
            &test_history("re-open-first"),
            std::io::Cursor::new("123456\n"),
            &mut first_output,
            now(),
        )
        .unwrap();

        let reopened = config::Config::load_from(&path).unwrap();
        let mut second_output = Vec::new();
        connect_and_write_report(
            reopened,
            &api_base(&server),
            &oauth_base(&server),
            &path,
            &test_history("re-open-second"),
            std::io::Cursor::new(""),
            &mut second_output,
            now(),
        )
        .unwrap();

        let text = String::from_utf8(second_output).unwrap();
        assert!(text.contains(&format!("@{username} · connected")));
        assert_eq!(request_token.calls(), 1);
        assert_eq!(access_token.calls(), 1);
        assert_eq!(users_me.calls(), 2);
    }

    #[test]
    fn first_run_exchange_failure_propagates_error() {
        let server = MockServer::start();
        mock_request_token(&server);
        mock_access_token(&server, 401);
        mock_users_me(&server, 200, "slickroot");

        let path = test_dir("exchange-failure").join("config.json");
        let mut output = Vec::new();

        let result = connect_and_write_report(
            unused_config(),
            &api_base(&server),
            &oauth_base(&server),
            &path,
            &test_history("exchange-failure"),
            std::io::Cursor::new("123456\n"),
            &mut output,
            now(),
        );

        assert!(result.is_err());
        let text = String::from_utf8(output).unwrap();
        assert!(!text.contains("connected"));
    }

    #[test]
    fn connected_path_prints_report_after_handle() {
        let server = MockServer::start();
        mock_users_me(&server, 200, "slickroot");
        mock_tweets(&server, 200, NO_TWEETS);
        let history = test_history("report");
        let date = now().date_naive().pred_opt().unwrap();
        let created_at: chrono::DateTime<chrono::Utc> = "2026-05-01T09:30:00Z".parse().unwrap();
        let time = created_at.with_timezone(&chrono::Local).format("%H:%M");
        history
            .save(
                date,
                &[model::Reply {
                    id: "7".into(),
                    created_at,
                    text: "hello there".into(),
                    to_username: "bob".into(),
                    impressions: 12,
                    likes: 3,
                    profile_visits: 4,
                }],
            )
            .unwrap();
        let mut output = Vec::new();

        connect_and_write_report(
            connected_config(),
            &api_base(&server),
            &oauth_base(&server),
            &test_dir("report-config").join("config.json"),
            &history,
            std::io::Cursor::new(""),
            &mut output,
            now(),
        )
        .unwrap();

        let text = String::from_utf8(output).unwrap();
        let chunks: Vec<&str> = text.split("\n\n---\n").collect();
        assert_eq!(chunks.len(), 5, "{text}");
        assert_eq!(chunks[0], "Welcome\n@slickroot · connected", "{text}");
        assert_eq!(
            chunks[1], "Last 30 days\n· · · · · · · · · · · · · · · · · · · · · · · · · · · · · ·",
            "{text}"
        );
        assert_eq!(
            chunks[2],
            format!(
                "Yesterday · 1 replies\n{time}  @bob  hello there                                          12 impressions  3 likes  4 profile visits  \x1b]8;;https://x.com/i/status/7\x1b\\[link]\x1b]8;;\x1b\\"
            ),
            "{text}"
        );
        assert_eq!(
            chunks[3],
            "Accounts\n1. \x1b]8;;https://x.com/bob\x1b\\@bob\x1b]8;;\x1b\\  12 avg impressions  1 replies",
            "{text}"
        );
        assert_eq!(chunks[4], "Today: 0 of 5 replies (5 to go)\n", "{text}");
        assert!(text.ends_with('\n'));
    }

    #[test]
    fn report_starts_with_welcome_then_grid() {
        let server = MockServer::start();
        mock_users_me(&server, 200, "slickroot");
        let today = now();
        let tweets: Vec<String> = (0..100)
            .map(|id| tweet_json(&id.to_string(), today.with_timezone(&chrono::Utc), "9"))
            .collect();
        mock_tweets(
            &server,
            200,
            &tweets_body(&tweets, &[included_user_json("9", "bob")]),
        );
        let mut output = Vec::new();

        connect_and_write_report(
            connected_config(),
            &api_base(&server),
            &oauth_base(&server),
            &test_dir("grid-config").join("config.json"),
            &test_history("grid-history"),
            std::io::Cursor::new(""),
            &mut output,
            today,
        )
        .unwrap();

        let text = String::from_utf8(output).unwrap();
        let chunks: Vec<&str> = text.split("\n\n---\n").collect();
        assert_eq!(chunks[0], "Welcome\n@slickroot · connected", "{text}");
        assert_eq!(
            chunks[1], "Last 30 days\n· · · · · · · · · · · · · · · · · · · · · · · · · · · · · ■",
            "{text}"
        );
    }

    #[test]
    fn empty_day_prints_no_replies_message() {
        let server = MockServer::start();
        mock_users_me(&server, 200, "slickroot");
        mock_tweets(&server, 200, NO_TWEETS);
        let mut output = Vec::new();

        connect_and_write_report(
            connected_config(),
            &api_base(&server),
            &oauth_base(&server),
            &test_dir("empty-day").join("config.json"),
            &test_history("empty-day"),
            std::io::Cursor::new(""),
            &mut output,
            now(),
        )
        .unwrap();

        let text = String::from_utf8(output).unwrap();
        assert!(text.contains("No replies yesterday.\n"), "{text}");
        assert!(
            text.ends_with("Today: 0 of 5 replies (5 to go)\n"),
            "{text}"
        );
    }

    #[test]
    fn accounts_section_appears_between_history_and_today_with_dividers() {
        let server = MockServer::start();
        mock_users_me(&server, 200, "slickroot");
        mock_tweets(&server, 200, NO_TWEETS);
        let history = test_history("accounts-section");
        let yesterday_date = now().date_naive().pred_opt().unwrap();
        let earlier_date = yesterday_date.pred_opt().unwrap();
        let created_at: chrono::DateTime<chrono::Utc> = "2026-05-01T09:30:00Z".parse().unwrap();
        let time = created_at.with_timezone(&chrono::Local).format("%H:%M");
        history
            .save(
                yesterday_date,
                &[model::Reply {
                    id: "7".into(),
                    created_at,
                    text: "hello there".into(),
                    to_username: "bob".into(),
                    impressions: 12,
                    likes: 3,
                    profile_visits: 4,
                }],
            )
            .unwrap();
        history
            .save(
                earlier_date,
                &[model::Reply {
                    id: "1".into(),
                    created_at: "2026-05-01T08:00:00Z".parse().unwrap(),
                    text: "hi".into(),
                    to_username: "alice".into(),
                    impressions: 20,
                    likes: 0,
                    profile_visits: 0,
                }],
            )
            .unwrap();
        let mut output = Vec::new();

        connect_and_write_report(
            connected_config(),
            &api_base(&server),
            &oauth_base(&server),
            &test_dir("accounts-section-config").join("config.json"),
            &history,
            std::io::Cursor::new(""),
            &mut output,
            now(),
        )
        .unwrap();

        let text = String::from_utf8(output).unwrap();
        let chunks: Vec<&str> = text.split("\n\n---\n").collect();
        assert_eq!(chunks.len(), 5, "{text}");
        assert_eq!(chunks[0], "Welcome\n@slickroot · connected", "{text}");
        assert_eq!(
            chunks[1], "Last 30 days\n· · · · · · · · · · · · · · · · · · · · · · · · · · · · · ·",
            "{text}"
        );
        assert_eq!(
            chunks[2],
            format!(
                "Yesterday · 1 replies\n{time}  @bob  hello there                                          12 impressions  3 likes  4 profile visits  \x1b]8;;https://x.com/i/status/7\x1b\\[link]\x1b]8;;\x1b\\"
            ),
            "{text}"
        );
        assert_eq!(
            chunks[3],
            "Accounts\n1. \x1b]8;;https://x.com/alice\x1b\\@alice\x1b]8;;\x1b\\  20 avg impressions  1 replies\n2. \x1b]8;;https://x.com/bob\x1b\\@bob\x1b]8;;\x1b\\    12 avg impressions  1 replies",
            "{text}"
        );
        assert_eq!(chunks[4], "Today: 0 of 5 replies (5 to go)\n", "{text}");
    }

    #[test]
    fn accounts_section_shows_no_data_yet_when_history_empty() {
        let server = MockServer::start();
        mock_users_me(&server, 200, "slickroot");
        mock_tweets(&server, 200, NO_TWEETS);
        let mut output = Vec::new();

        connect_and_write_report(
            connected_config(),
            &api_base(&server),
            &oauth_base(&server),
            &test_dir("accounts-empty").join("config.json"),
            &test_history("accounts-empty"),
            std::io::Cursor::new(""),
            &mut output,
            now(),
        )
        .unwrap();

        let text = String::from_utf8(output).unwrap();
        let chunks: Vec<&str> = text.split("\n\n---\n").collect();
        assert_eq!(chunks.len(), 5, "{text}");
        assert_eq!(chunks[0], "Welcome\n@slickroot · connected", "{text}");
        assert_eq!(
            chunks[1], "Last 30 days\n· · · · · · · · · · · · · · · · · · · · · · · · · · · · · ·",
            "{text}"
        );
        assert_eq!(
            chunks[2], "Yesterday · 0 replies\nNo replies yesterday.",
            "{text}"
        );
        assert_eq!(chunks[3], "Accounts\nNo data yet.", "{text}");
        assert_eq!(chunks[4], "Today: 0 of 5 replies (5 to go)\n", "{text}");
    }

    fn seeded_history(name: &str) -> history::History {
        let history = test_history(name);
        let date = now().date_naive().pred_opt().unwrap();
        history.save(date, &[]).unwrap();
        history
    }

    #[test]
    fn today_count_comes_from_todays_history_file() {
        let server = MockServer::start();
        mock_users_me(&server, 200, "slickroot");
        let today = now();
        let body = tweets_body(
            &[tweet_json("42", today.with_timezone(&chrono::Utc), "9")],
            &[included_user_json("9", "bob")],
        );
        mock_tweets(&server, 200, &body);
        let mut output = Vec::new();

        connect_and_write_report(
            connected_config(),
            &api_base(&server),
            &oauth_base(&server),
            &test_dir("today-cached-config").join("config.json"),
            &seeded_history("today-cached-history"),
            std::io::Cursor::new(""),
            &mut output,
            today,
        )
        .unwrap();

        let text = String::from_utf8(output).unwrap();
        assert!(text.contains("Today: 1 of 5 replies (4 to go)"), "{text}");
    }

    #[test]
    fn connect_first_run_runs_pin_flow_and_returns_working_client() {
        let server = MockServer::start();
        mock_request_token(&server);
        mock_access_token(&server, 200);
        let username = "slickroot";
        mock_users_me(&server, 200, username);

        let path = test_dir("connect-first-run").join("config.json");
        let mut output = Vec::new();

        let (config, api, me) = connect(
            unused_config(),
            &api_base(&server),
            &oauth_base(&server),
            &path,
            std::io::Cursor::new("123456\n"),
            &mut output,
        )
        .unwrap();

        assert!(config.is_connected());
        assert_eq!(me.handle, format!("@{username}"));
        assert!(api.users_me().is_ok());
    }

    #[test]
    fn connect_already_connected_skips_oauth_and_returns_working_client() {
        let server = MockServer::start();
        let request_token = mock_request_token(&server);
        let access_token = mock_access_token(&server, 200);
        let username = "slickroot";
        mock_users_me(&server, 200, username);

        let path = test_dir("connect-already-connected").join("config.json");
        let mut output = Vec::new();

        let (config, api, me) = connect(
            connected_config(),
            &api_base(&server),
            &oauth_base(&server),
            &path,
            std::io::Cursor::new(""),
            &mut output,
        )
        .unwrap();

        assert_eq!(request_token.calls(), 0);
        assert_eq!(access_token.calls(), 0);
        assert!(config.is_connected());
        assert_eq!(me.handle, format!("@{username}"));
        assert!(api.users_me().is_ok());
        assert!(output.is_empty());
    }

    fn reply_from(id: &str, created_at: chrono::DateTime<chrono::Utc>) -> model::Reply {
        model::Reply {
            id: id.into(),
            created_at,
            text: "hi".into(),
            to_username: "bob".into(),
            impressions: 1,
            likes: 2,
            profile_visits: 3,
        }
    }

    #[test]
    fn save_replies_saves_each_reply_under_its_local_posted_day() {
        let history = test_history("save-replies-empty-history");
        let now = now();
        let today = now.date_naive();
        let two_days_ago = today.pred_opt().unwrap().pred_opt().unwrap();

        let today_reply = reply_from("1", now.with_timezone(&chrono::Utc));
        let earlier_reply = reply_from(
            "2",
            two_days_ago
                .and_hms_opt(10, 0, 0)
                .unwrap()
                .and_local_timezone(chrono::Local)
                .unwrap()
                .with_timezone(&chrono::Utc),
        );

        save_replies(&history, vec![today_reply, earlier_reply]).unwrap();

        let today_saved = history.load(today).unwrap().unwrap();
        assert_eq!(
            today_saved
                .iter()
                .map(|r| r.id.as_str())
                .collect::<Vec<_>>(),
            vec!["1"]
        );

        let earlier_saved = history.load(two_days_ago).unwrap().unwrap();
        assert_eq!(
            earlier_saved
                .iter()
                .map(|r| r.id.as_str())
                .collect::<Vec<_>>(),
            vec!["2"]
        );
    }

    #[test]
    fn save_replies_leaves_a_day_absent_from_the_batch_untouched() {
        let history = test_history("save-replies-day-absent-from-batch");
        let today = now().date_naive();
        let untouched_day = today.pred_opt().unwrap().pred_opt().unwrap();

        let preexisting_created_at = untouched_day
            .and_hms_opt(9, 0, 0)
            .unwrap()
            .and_local_timezone(chrono::Local)
            .unwrap()
            .with_timezone(&chrono::Utc);
        history
            .save(untouched_day, &[reply_from("1", preexisting_created_at)])
            .unwrap();

        let today_reply = reply_from(
            "2",
            today
                .and_hms_opt(9, 0, 0)
                .unwrap()
                .and_local_timezone(chrono::Local)
                .unwrap()
                .with_timezone(&chrono::Utc),
        );
        save_replies(&history, vec![today_reply]).unwrap();

        let untouched_saved = history.load(untouched_day).unwrap().unwrap();
        assert_eq!(
            untouched_saved,
            vec![reply_from("1", preexisting_created_at)]
        );
    }

    #[test]
    fn fetch_spanning_a_skipped_day_saves_every_reply_under_its_own_day() {
        let server = MockServer::start();
        mock_users_me(&server, 200, "slickroot");
        let now = now();
        let today = now.date_naive();
        let skipped_day = today.pred_opt().unwrap().pred_opt().unwrap();
        let skipped_day_created_at = skipped_day
            .and_hms_opt(9, 0, 0)
            .unwrap()
            .and_local_timezone(chrono::Local)
            .unwrap()
            .with_timezone(&chrono::Utc);

        let body = tweets_body(
            &[
                tweet_json("1", now.with_timezone(&chrono::Utc), "9"),
                tweet_json("2", skipped_day_created_at, "10"),
            ],
            &[
                included_user_json("9", "carol"),
                included_user_json("10", "dave"),
            ],
        );
        mock_tweets(&server, 200, &body);

        let history = test_history("acceptance-skipped-day");
        let mut output = Vec::new();

        connect_and_write_report(
            connected_config(),
            &api_base(&server),
            &oauth_base(&server),
            &test_dir("acceptance-skipped-day-config").join("config.json"),
            &history,
            std::io::Cursor::new(""),
            &mut output,
            now,
        )
        .unwrap();

        let today_saved = history.load(today).unwrap().unwrap();
        assert_eq!(
            today_saved
                .iter()
                .map(|r| r.id.as_str())
                .collect::<Vec<_>>(),
            vec!["1"]
        );

        let skipped_day_saved = history.load(skipped_day).unwrap().unwrap();
        assert_eq!(
            skipped_day_saved
                .iter()
                .map(|r| r.id.as_str())
                .collect::<Vec<_>>(),
            vec!["2"]
        );

        let text = String::from_utf8(output).unwrap();
        assert!(text.contains("@dave"), "{text}");

        assert!(text.contains("Today: 1 of 5 replies (4 to go)"), "{text}");
    }
}
