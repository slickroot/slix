mod accounts;
mod api;
mod config;
mod history;
mod oauth;
mod report;
mod today;
mod window;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = config::Config::load()?;
    let api_base = "https://api.x.com/2/";
    let oauth_base = "https://api.x.com/oauth";
    let path = config::Config::path();
    let history = history::History::new(config.data_dir.join("history"));
    let today_goal = today::TodayGoal::new(config.data_dir.join("today.json"));
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
            write_report(
                &api,
                &me,
                &history,
                &today_goal,
                chrono::Local::now(),
                &mut output,
            )?;
        }
        Err(err) => match err.downcast::<Reconnected>() {
            Ok(_) => {}
            Err(err) => return Err(err),
        },
    }
    Ok(())
}

fn write_report(
    api: &api::XApiClient,
    me: &api::Me,
    history: &history::History,
    today_goal: &today::TodayGoal,
    now: chrono::DateTime<chrono::Local>,
    output: &mut impl std::io::Write,
) -> Result<(), Box<dyn std::error::Error>> {
    writeln!(output, "{} · connected", me.handle)?;
    let date = now.date_naive().pred_opt().unwrap();
    let latest = api.latest_replies(&me.id)?;
    let today_count = match today_goal.load(now)? {
        Some(count) => count,
        None => {
            let today = window::Window::today(now);
            let count = latest
                .iter()
                .filter(|reply| today.contains(&reply.created_at))
                .count() as u64;
            today_goal.save(count, now)?;
            count
        }
    };
    let replies = match history.load(date)? {
        Some(replies) => replies,
        None => {
            let yesterday = window::Window::yesterday(now);
            let replies: Vec<api::Reply> = latest
                .into_iter()
                .filter(|reply| yesterday.contains(&reply.created_at))
                .collect();
            history.save(date, &replies)?;
            replies
        }
    };
    writeln!(output, "{}", report::render(&replies, &chrono::Local))?;

    writeln!(output)?;
    writeln!(output, "---")?;
    writeln!(output)?;
    let all_replies = history.load_all()?;
    let ranks = accounts::rank(&all_replies);
    writeln!(output, "{}", report::render_accounts(&ranks))?;
    writeln!(output)?;
    writeln!(output, "---")?;
    writeln!(output)?;

    writeln!(output, "{}", report::render_today(today_count))?;
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
) -> Result<(config::Config, api::XApiClient, api::Me), Box<dyn std::error::Error>> {
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
        today_goal: &today::TodayGoal,
        input: impl std::io::BufRead,
        mut output: impl std::io::Write,
        now: chrono::DateTime<chrono::Local>,
    ) -> Result<config::Config, Box<dyn std::error::Error>> {
        match connect(config, api_base, oauth_base, path, input, &mut output) {
            Ok((config, api, me)) => {
                write_report(&api, &me, history, today_goal, now, &mut output)?;
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

    fn test_today_goal(name: &str) -> today::TodayGoal {
        today::TodayGoal::new(test_dir(name).join("today.json"))
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

    fn now() -> chrono::DateTime<chrono::Local> {
        chrono::Local::now()
    }

    fn one_reply_body(created_at: chrono::DateTime<chrono::Local>) -> String {
        format!(
            r#"{{"data":[{{"id":"7","created_at":"{}","text":"hello there","public_metrics":{{"impression_count":12,"like_count":3}},"non_public_metrics":{{"user_profile_clicks":4}},"referenced_tweets":[{{"type":"replied_to","id":"1"}}],"in_reply_to_user_id":"9"}}],"includes":{{"users":[{{"id":"9","username":"bob"}}]}}}}"#,
            created_at
                .with_timezone(&chrono::Utc)
                .to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
        )
    }

    fn yesterday() -> chrono::DateTime<chrono::Local> {
        now() - chrono::Duration::days(1)
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
            &test_today_goal("first-run"),
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
            &test_today_goal("verify-on-startup"),
            std::io::Cursor::new(""),
            &mut output,
            now(),
        )
        .unwrap();

        let text = String::from_utf8(output).unwrap();
        assert_eq!(
            text.lines().next().unwrap(),
            format!("@{username} · connected")
        );
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
            &test_today_goal("reconnect"),
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
            &test_today_goal("re-open-first"),
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
            &test_today_goal("re-open-second"),
            std::io::Cursor::new(""),
            &mut second_output,
            now(),
        )
        .unwrap();

        let text = String::from_utf8(second_output).unwrap();
        assert_eq!(
            text.lines().next().unwrap(),
            format!("@{username} · connected")
        );
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
            &test_today_goal("exchange-failure"),
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
        mock_tweets(&server, 200, &one_reply_body(yesterday()));
        let mut output = Vec::new();

        connect_and_write_report(
            connected_config(),
            &api_base(&server),
            &oauth_base(&server),
            &test_dir("report").join("config.json"),
            &test_history("report"),
            &test_today_goal("report"),
            std::io::Cursor::new(""),
            &mut output,
            now(),
        )
        .unwrap();

        let text = String::from_utf8(output).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines[0], "@slickroot · connected");
        assert_eq!(lines[1], "Yesterday · 1 replies");
        assert!(lines[2].contains("@bob"), "{text}");
        assert!(lines[2].contains("12 impressions"), "{text}");
        assert_eq!(lines[3], "");
        assert_eq!(lines[4], "---");
        assert_eq!(lines[5], "");
        assert_eq!(lines[6], "Accounts");
        assert!(lines[7].contains("@bob"), "{text}");
        assert_eq!(lines[8], "");
        assert_eq!(lines[9], "---");
        assert_eq!(lines[10], "");
        assert_eq!(lines[11], report::render_today(0));
        assert_eq!(lines.len(), 12, "{text}");
        assert!(text.ends_with('\n'));
    }

    #[test]
    fn one_fetch_fills_yesterday_and_today() {
        let server = MockServer::start();
        mock_users_me(&server, 200, "slickroot");
        let now = now();
        let reply = |id: &str, created_at: chrono::DateTime<chrono::Local>| {
            format!(
                r#"{{"id":"{id}","created_at":"{}","text":"hello there","public_metrics":{{"impression_count":12,"like_count":3}},"non_public_metrics":{{"user_profile_clicks":4}},"referenced_tweets":[{{"type":"replied_to","id":"1"}}],"in_reply_to_user_id":"9"}}"#,
                created_at
                    .with_timezone(&chrono::Utc)
                    .to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            )
        };
        let body = format!(
            r#"{{"data":[{},{}],"includes":{{"users":[{{"id":"9","username":"bob"}}]}}}}"#,
            reply("7", now - chrono::Duration::days(1)),
            reply("8", now),
        );
        let tweets = mock_tweets(&server, 200, &body);
        let mut output = Vec::new();

        connect_and_write_report(
            connected_config(),
            &api_base(&server),
            &oauth_base(&server),
            &test_dir("one-fetch").join("config.json"),
            &test_history("one-fetch"),
            &test_today_goal("one-fetch"),
            std::io::Cursor::new(""),
            &mut output,
            now,
        )
        .unwrap();

        let text = String::from_utf8(output).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(tweets.calls(), 1);
        assert_eq!(lines[1], "Yesterday · 1 replies");
        assert_eq!(
            lines.last().copied(),
            Some(report::render_today(1).as_str())
        );
    }

    #[test]
    fn first_run_of_day_saves_fetched_replies_to_history() {
        let server = MockServer::start();
        mock_users_me(&server, 200, "slickroot");
        let tweets = mock_tweets(&server, 200, &one_reply_body(yesterday()));
        let history = test_history("save-on-fetch");
        let today_goal = test_today_goal("save-on-fetch");

        connect_and_write_report(
            connected_config(),
            &api_base(&server),
            &oauth_base(&server),
            &test_dir("save-on-fetch").join("config.json"),
            &history,
            &today_goal,
            std::io::Cursor::new(""),
            &mut Vec::new(),
            now(),
        )
        .unwrap();

        assert_eq!(tweets.calls(), 1);
        let date = now().date_naive().pred_opt().unwrap();
        let saved = history.load(date).unwrap().unwrap();
        assert_eq!(saved.len(), 1);
        assert_eq!(saved[0].id, "7");
    }

    #[test]
    fn later_run_same_day_loads_yesterday_from_history() {
        let server = MockServer::start();
        mock_users_me(&server, 200, "slickroot");
        let tweets = mock_tweets(&server, 200, &one_reply_body(yesterday()));
        let history = test_history("cached");
        let today_goal = test_today_goal("cached");
        let path = test_dir("cached").join("config.json");

        connect_and_write_report(
            connected_config(),
            &api_base(&server),
            &oauth_base(&server),
            &path,
            &history,
            &today_goal,
            std::io::Cursor::new(""),
            &mut Vec::new(),
            now(),
        )
        .unwrap();

        let second_server = MockServer::start();
        mock_users_me(&second_server, 200, "slickroot");
        let second_tweets = mock_tweets(&second_server, 200, NO_TWEETS);
        let mut second_output = Vec::new();
        connect_and_write_report(
            connected_config(),
            &api_base(&second_server),
            &oauth_base(&second_server),
            &path,
            &history,
            &today_goal,
            std::io::Cursor::new(""),
            &mut second_output,
            now(),
        )
        .unwrap();

        assert_eq!(tweets.calls(), 1);
        assert_eq!(second_tweets.calls(), 1);
        let text = String::from_utf8(second_output).unwrap();
        assert!(text.contains("@bob"), "{text}");
        assert!(text.contains("12 impressions"), "{text}");
    }

    #[test]
    fn api_failure_on_first_run_saves_nothing_and_retries_next_run() {
        let history = test_history("api-failure");
        let date = now().date_naive().pred_opt().unwrap();

        let failing_server = MockServer::start();
        mock_users_me(&failing_server, 200, "slickroot");
        let failing_tweets = mock_tweets(&failing_server, 500, "boom");

        let result = connect_and_write_report(
            connected_config(),
            &api_base(&failing_server),
            &oauth_base(&failing_server),
            &test_dir("api-failure").join("config.json"),
            &history,
            &test_today_goal("api-failure"),
            std::io::Cursor::new(""),
            &mut Vec::new(),
            now(),
        );

        assert!(result.is_err());
        assert_eq!(failing_tweets.calls(), 1);
        assert!(history.load(date).unwrap().is_none());

        let succeeding_server = MockServer::start();
        mock_users_me(&succeeding_server, 200, "slickroot");
        let tweets = mock_tweets(&succeeding_server, 200, NO_TWEETS);

        connect_and_write_report(
            connected_config(),
            &api_base(&succeeding_server),
            &oauth_base(&succeeding_server),
            &test_dir("api-failure-retry").join("config.json"),
            &history,
            &test_today_goal("api-failure-retry"),
            std::io::Cursor::new(""),
            &mut Vec::new(),
            now(),
        )
        .unwrap();

        assert_eq!(tweets.calls(), 1);
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
            &test_today_goal("empty-day"),
            std::io::Cursor::new(""),
            &mut output,
            now(),
        )
        .unwrap();

        let text = String::from_utf8(output).unwrap();
        assert!(text.contains("No replies yesterday.\n"), "{text}");
        assert!(
            text.ends_with(&format!("{}\n", report::render_today(0))),
            "{text}"
        );
    }

    #[test]
    fn accounts_section_appears_between_history_and_today_with_dividers() {
        let server = MockServer::start();
        mock_users_me(&server, 200, "slickroot");
        mock_tweets(&server, 200, &one_reply_body(yesterday()));
        let history = test_history("accounts-section");
        let earlier_date = now().date_naive().pred_opt().unwrap().pred_opt().unwrap();
        history
            .save(
                earlier_date,
                &[api::Reply {
                    id: "1".into(),
                    created_at: chrono::Utc::now(),
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
            &test_today_goal("accounts-section-today"),
            std::io::Cursor::new(""),
            &mut output,
            now(),
        )
        .unwrap();

        let text = String::from_utf8(output).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines[0], "@slickroot · connected");
        assert_eq!(lines[1], "Yesterday · 1 replies");
        assert!(lines[2].contains("@bob"), "{text}");
        assert_eq!(lines[3], "");
        assert_eq!(lines[4], "---");
        assert_eq!(lines[5], "");
        assert_eq!(lines[6], "Accounts");
        assert!(lines[7].contains("@alice"), "{text}");
        assert!(lines[8].contains("@bob"), "{text}");
        assert_eq!(lines[9], "");
        assert_eq!(lines[10], "---");
        assert_eq!(lines[11], "");
        assert_eq!(lines[12], report::render_today(0));
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
            &test_today_goal("accounts-empty"),
            std::io::Cursor::new(""),
            &mut output,
            now(),
        )
        .unwrap();

        let text = String::from_utf8(output).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines[0], "@slickroot · connected");
        assert_eq!(lines[1], "Yesterday · 0 replies");
        assert_eq!(lines[2], "No replies yesterday.");
        assert_eq!(lines[3], "");
        assert_eq!(lines[4], "---");
        assert_eq!(lines[5], "");
        assert_eq!(lines[6], "Accounts");
        assert_eq!(lines[7], "No data yet.");
        assert_eq!(lines[8], "");
        assert_eq!(lines[9], "---");
        assert_eq!(lines[10], "");
        assert_eq!(lines[11], report::render_today(0));
    }

    #[test]
    fn failed_replies_fetch_propagates_error() {
        let server = MockServer::start();
        mock_users_me(&server, 200, "slickroot");
        mock_tweets(&server, 500, "boom");

        let result = connect_and_write_report(
            connected_config(),
            &api_base(&server),
            &oauth_base(&server),
            &test_dir("replies-failure").join("config.json"),
            &test_history("replies-failure"),
            &test_today_goal("replies-failure"),
            std::io::Cursor::new(""),
            &mut Vec::new(),
            now(),
        );

        assert!(result.is_err());
    }

    fn seeded_history(name: &str) -> history::History {
        let history = test_history(name);
        let date = now().date_naive().pred_opt().unwrap();
        history.save(date, &[]).unwrap();
        history
    }

    #[test]
    fn no_cached_today_count_fetches_from_api_saves_it_and_prints_today_line() {
        let server = MockServer::start();
        mock_users_me(&server, 200, "slickroot");
        let tweets = mock_tweets(&server, 200, &one_reply_body(now()));
        let today_goal = test_today_goal("today-fetch-goal");
        let mut output = Vec::new();

        connect_and_write_report(
            connected_config(),
            &api_base(&server),
            &oauth_base(&server),
            &test_dir("today-fetch-config").join("config.json"),
            &seeded_history("today-fetch-history"),
            &today_goal,
            std::io::Cursor::new(""),
            &mut output,
            now(),
        )
        .unwrap();

        assert_eq!(tweets.calls(), 1);
        assert_eq!(today_goal.load(now()).unwrap(), Some(1));
        let text = String::from_utf8(output).unwrap();
        assert!(text.contains(&report::render_today(1)), "{text}");
    }

    #[test]
    fn cached_fresh_today_count_is_reused_over_fetched_posts() {
        let server = MockServer::start();
        mock_users_me(&server, 200, "slickroot");
        let tweets = mock_tweets(&server, 200, &one_reply_body(now()));
        let today_goal = test_today_goal("today-cached-goal");
        today_goal.save(3, now()).unwrap();
        let mut output = Vec::new();

        connect_and_write_report(
            connected_config(),
            &api_base(&server),
            &oauth_base(&server),
            &test_dir("today-cached-config").join("config.json"),
            &seeded_history("today-cached-history"),
            &today_goal,
            std::io::Cursor::new(""),
            &mut output,
            now(),
        )
        .unwrap();

        assert_eq!(tweets.calls(), 1);
        let text = String::from_utf8(output).unwrap();
        assert!(text.contains(&report::render_today(3)), "{text}");
        assert!(!text.contains(&report::render_today(1)), "{text}");
    }

    #[test]
    fn today_fetch_failure_saves_nothing_and_propagates_error() {
        let server = MockServer::start();
        mock_users_me(&server, 200, "slickroot");
        mock_tweets(&server, 500, "boom");
        let today_goal = test_today_goal("today-failure-goal");

        let result = connect_and_write_report(
            connected_config(),
            &api_base(&server),
            &oauth_base(&server),
            &test_dir("today-failure-config").join("config.json"),
            &seeded_history("today-failure-history"),
            &today_goal,
            std::io::Cursor::new(""),
            &mut Vec::new(),
            now(),
        );

        assert!(result.is_err());
        assert!(today_goal.load(now()).unwrap().is_none());
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
}
