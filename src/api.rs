use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt;

use chrono::{DateTime, SecondsFormat, Utc};

use oauth1::Token;
use reqwest::blocking::Client;
use reqwest::header::AUTHORIZATION;
use serde::Deserialize;

use crate::window::Window;

pub struct XApiClient {
    base: String,
    endpoint: String,
    consumer: Token<'static>,
    access_token: Token<'static>,
    client: Client,
}

#[derive(Debug)]
pub enum ApiError {
    NeedsReconnect,
    Http(reqwest::Error),
    Protocol(String),
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NeedsReconnect => {
                write!(f, "stored tokens no longer accepted; reconnect required")
            }
            Self::Http(e) => write!(f, "api http error: {e}"),
            Self::Protocol(msg) => write!(f, "api protocol error: {msg}"),
        }
    }
}

impl std::error::Error for ApiError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Http(e) => Some(e),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub struct Me {
    pub id: String,
    pub handle: String,
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct Reply {
    pub id: String,
    pub created_at: DateTime<Utc>,
    pub text: String,
    pub to_username: String,
    pub impressions: u64,
}

#[derive(Deserialize)]
struct TweetsResponse {
    #[serde(default)]
    data: Vec<Tweet>,
}

#[derive(Deserialize)]
struct Tweet {
    id: String,
    created_at: DateTime<Utc>,
    text: String,
    public_metrics: PublicMetrics,
}

#[derive(Deserialize)]
struct PublicMetrics {
    impression_count: u64,
}

#[derive(Deserialize)]
struct UserMe {
    data: UserMeData,
}

#[derive(Deserialize)]
struct UserMeData {
    id: String,
    username: String,
}

impl XApiClient {
    pub fn for_endpoint(
        base_url: &str,
        consumer_key: &str,
        consumer_secret: &str,
        access_token: &str,
        access_token_secret: &str,
        client: Client,
    ) -> Self {
        let base = base_url.trim_end_matches('/');
        Self {
            base: base.to_string(),
            endpoint: format!("{base}/users/me"),
            consumer: Token::new(consumer_key.to_string(), consumer_secret.to_string()),
            access_token: Token::new(access_token.to_string(), access_token_secret.to_string()),
            client,
        }
    }

    pub fn users_me(&self) -> Result<Me, ApiError> {
        let authorization = oauth1::authorize(
            "GET",
            &self.endpoint,
            &self.consumer,
            Some(&self.access_token),
            None,
        );
        let response = self
            .client
            .get(&self.endpoint)
            .header(AUTHORIZATION, authorization)
            .send()
            .map_err(ApiError::Http)?;
        let status = response.status();
        if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
            return Err(ApiError::NeedsReconnect);
        }
        let body = response
            .error_for_status()
            .map_err(ApiError::Http)?
            .text()
            .map_err(ApiError::Http)?;
        let me: UserMe =
            serde_json::from_str(&body).map_err(|e| ApiError::Protocol(e.to_string()))?;
        Ok(Me {
            id: me.data.id,
            handle: format!("@{}", me.data.username),
        })
    }

    #[allow(dead_code)]
    pub fn replies(&self, user_id: &str, window: &Window) -> Result<Vec<Reply>, ApiError> {
        let url = format!("{}/users/{user_id}/tweets", self.base);
        let query = [
            (
                "start_time",
                window.start.to_rfc3339_opts(SecondsFormat::Secs, true),
            ),
            (
                "end_time",
                window.end.to_rfc3339_opts(SecondsFormat::Secs, true),
            ),
            ("max_results", "100".to_string()),
            ("exclude", "retweets".to_string()),
            (
                "tweet.fields",
                "created_at,public_metrics,referenced_tweets,in_reply_to_user_id".to_string(),
            ),
            ("expansions", "in_reply_to_user_id".to_string()),
        ];
        let signed_params: HashMap<&str, Cow<str>> = query
            .iter()
            .map(|(name, value)| (*name, Cow::Borrowed(value.as_str())))
            .collect();
        let authorization = oauth1::authorize(
            "GET",
            &url,
            &self.consumer,
            Some(&self.access_token),
            Some(signed_params),
        );
        let body = self
            .client
            .get(&url)
            .query(&query)
            .header(AUTHORIZATION, authorization)
            .send()
            .map_err(ApiError::Http)?
            .text()
            .map_err(ApiError::Http)?;
        let tweets: TweetsResponse =
            serde_json::from_str(&body).map_err(|e| ApiError::Protocol(e.to_string()))?;
        Ok(tweets
            .data
            .into_iter()
            .map(|tweet| Reply {
                id: tweet.id,
                created_at: tweet.created_at,
                text: tweet.text,
                to_username: String::new(),
                impressions: tweet.public_metrics.impression_count,
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use httpmock::MockServer;

    fn api_client(server: &MockServer) -> XApiClient {
        XApiClient::for_endpoint(
            &format!("{}/2/", server.base_url()),
            "test-consumer-key",
            "test-consumer-secret",
            "test-access-token",
            "test-access-token-secret",
            Client::new(),
        )
    }

    #[test]
    fn users_me_returns_id_and_handle_with_at_prefix() {
        let server = MockServer::start();
        let id = "2244994945";
        let username = "slickroot";
        server.mock(|when, then| {
            when.method(httpmock::Method::GET).path("/2/users/me");
            then.status(200).body(format!(
                r#"{{"data":{{"id":"{id}","name":"Slick Root","username":"{username}"}}}}"#
            ));
        });

        let client = api_client(&server);
        let me = client.users_me().unwrap();

        assert_eq!(me.id, id);
        assert_eq!(me.handle, format!("@{username}"));
    }

    #[test]
    fn users_me_maps_missing_id_to_protocol_error() {
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(httpmock::Method::GET).path("/2/users/me");
            then.status(200)
                .body(r#"{"data":{"name":"Slick Root","username":"slickroot"}}"#);
        });

        let client = api_client(&server);
        let err = client.users_me().unwrap_err();

        assert!(matches!(err, ApiError::Protocol(_)), "{err:?}");
    }

    #[test]
    fn users_me_sends_signed_get_to_users_me_endpoint() {
        let server = MockServer::start();
        let seen_auth = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
        let seen_auth_for_mock = std::sync::Arc::clone(&seen_auth);
        server.mock(|when, then| {
            when.method(httpmock::Method::GET).path("/2/users/me");
            then.respond_with(move |req: &httpmock::HttpMockRequest| {
                *seen_auth_for_mock.lock().unwrap() = req
                    .headers()
                    .get("authorization")
                    .map(|value| value.to_str().unwrap().to_string())
                    .unwrap_or_default();
                httpmock::HttpMockResponse::builder()
                    .status(200)
                    .body(r#"{"data":{"id":"1","name":"Slick Root","username":"slickroot"}}"#)
                    .build()
            });
        });

        let client = api_client(&server);
        client.users_me().unwrap();

        let auth = seen_auth.lock().unwrap();
        assert!(auth.starts_with("OAuth "), "{auth}");
        assert!(auth.contains("oauth_token=\"test-access-token\""), "{auth}");
    }

    #[test]
    fn users_me_maps_401_to_needs_reconnect() {
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(httpmock::Method::GET).path("/2/users/me");
            then.status(401).body("unauthorized");
        });

        let client = api_client(&server);
        let err = client.users_me().unwrap_err();

        assert!(matches!(err, ApiError::NeedsReconnect), "{err:?}");
    }

    #[test]
    fn users_me_maps_403_to_needs_reconnect() {
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(httpmock::Method::GET).path("/2/users/me");
            then.status(403).body("forbidden");
        });

        let client = api_client(&server);
        let err = client.users_me().unwrap_err();

        assert!(matches!(err, ApiError::NeedsReconnect), "{err:?}");
    }

    #[test]
    fn users_me_maps_missing_username_to_protocol_error() {
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(httpmock::Method::GET).path("/2/users/me");
            then.status(200)
                .body(r#"{"data":{"id":"1","name":"Slick Root"}}"#);
        });

        let client = api_client(&server);
        let err = client.users_me().unwrap_err();

        assert!(matches!(err, ApiError::Protocol(_)), "{err:?}");
    }

    #[test]
    fn users_me_maps_500_to_http_error() {
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(httpmock::Method::GET).path("/2/users/me");
            then.status(500).body("boom");
        });

        let client = api_client(&server);
        let err = client.users_me().unwrap_err();

        assert!(matches!(err, ApiError::Http(_)), "{err:?}");
    }

    #[test]
    fn replies_sends_signed_get_with_window_and_field_params() {
        use chrono::TimeZone;
        let server = MockServer::start();
        let seen_auth = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
        let seen_auth_for_mock = std::sync::Arc::clone(&seen_auth);
        let mock = server.mock(|when, then| {
            when.method(httpmock::Method::GET)
                .path("/2/users/42/tweets")
                .query_param("start_time", "2026-09-20T00:00:00Z")
                .query_param("end_time", "2026-09-21T00:00:00Z")
                .query_param("max_results", "100")
                .query_param("exclude", "retweets")
                .query_param(
                    "tweet.fields",
                    "created_at,public_metrics,referenced_tweets,in_reply_to_user_id",
                )
                .query_param("expansions", "in_reply_to_user_id");
            then.respond_with(move |req: &httpmock::HttpMockRequest| {
                *seen_auth_for_mock.lock().unwrap() = req
                    .headers()
                    .get("authorization")
                    .map(|value| value.to_str().unwrap().to_string())
                    .unwrap_or_default();
                httpmock::HttpMockResponse::builder()
                    .status(200)
                    .body(
                        r#"{"data":[{"id":"7","created_at":"2026-09-20T10:15:00.000Z","text":"hi","public_metrics":{"impression_count":12}}]}"#,
                    )
                    .build()
            });
        });
        let window = Window {
            start: Utc.with_ymd_and_hms(2026, 9, 20, 0, 0, 0).unwrap(),
            end: Utc.with_ymd_and_hms(2026, 9, 21, 0, 0, 0).unwrap(),
        };

        let replies = api_client(&server).replies("42", &window).unwrap();

        mock.assert();
        assert_eq!(replies.len(), 1);
        assert_eq!(replies[0].id, "7");
        assert_eq!(replies[0].impressions, 12);
        let auth = seen_auth.lock().unwrap();
        assert!(auth.starts_with("OAuth "), "{auth}");
        assert!(auth.contains("oauth_token=\"test-access-token\""), "{auth}");
        assert!(auth.contains("oauth_signature=\""), "{auth}");
    }
}
