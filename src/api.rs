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
pub struct Reply {
    pub id: String,
    pub created_at: DateTime<Utc>,
    pub text: String,
    pub to_username: String,
    pub impressions: u64,
    pub likes: u64,
    pub profile_visits: u64,
}

#[derive(Deserialize)]
struct TweetsResponse {
    #[serde(default)]
    data: Vec<Tweet>,
    #[serde(default)]
    includes: Includes,
}

#[derive(Deserialize, Default)]
struct Includes {
    #[serde(default)]
    users: Vec<IncludedUser>,
}

#[derive(Deserialize)]
struct IncludedUser {
    id: String,
    username: String,
}

#[derive(Deserialize)]
struct ReferencedTweet {
    #[serde(rename = "type")]
    kind: String,
}

#[derive(Deserialize)]
struct Tweet {
    id: String,
    created_at: DateTime<Utc>,
    text: String,
    public_metrics: PublicMetrics,
    non_public_metrics: NonPublicMetrics,
    #[serde(default)]
    referenced_tweets: Vec<ReferencedTweet>,
    in_reply_to_user_id: Option<String>,
}

#[derive(Deserialize)]
struct PublicMetrics {
    impression_count: u64,
    like_count: u64,
}

#[derive(Deserialize)]
struct NonPublicMetrics {
    user_profile_clicks: u64,
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
                "created_at,public_metrics,non_public_metrics,referenced_tweets,in_reply_to_user_id".to_string(),
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
        let response = self
            .client
            .get(&url)
            .query(&query)
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
        let tweets: TweetsResponse =
            serde_json::from_str(&body).map_err(|e| ApiError::Protocol(e.to_string()))?;
        let usernames: HashMap<&str, &str> = tweets
            .includes
            .users
            .iter()
            .map(|user| (user.id.as_str(), user.username.as_str()))
            .collect();
        Ok(tweets
            .data
            .iter()
            .filter(|tweet| {
                tweet
                    .referenced_tweets
                    .iter()
                    .any(|referenced| referenced.kind == "replied_to")
            })
            .map(|tweet| Reply {
                id: tweet.id.clone(),
                created_at: tweet.created_at,
                text: tweet.text.clone(),
                to_username: tweet
                    .in_reply_to_user_id
                    .as_deref()
                    .and_then(|id| usernames.get(id))
                    .map(|username| username.to_string())
                    .unwrap_or_default(),
                impressions: tweet.public_metrics.impression_count,
                likes: tweet.public_metrics.like_count,
                profile_visits: tweet.non_public_metrics.user_profile_clicks,
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
                    "created_at,public_metrics,non_public_metrics,referenced_tweets,in_reply_to_user_id",
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
                        r#"{"data":[{"id":"7","created_at":"2026-09-20T10:15:00.000Z","text":"hi","public_metrics":{"impression_count":12,"like_count":3},"non_public_metrics":{"user_profile_clicks":4},"referenced_tweets":[{"type":"replied_to","id":"1"}],"in_reply_to_user_id":"9"}],"includes":{"users":[{"id":"9","username":"bob"}]}}"#,
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
        assert_eq!(replies[0].likes, 3);
        assert_eq!(replies[0].profile_visits, 4);
        let auth = seen_auth.lock().unwrap();
        assert!(auth.starts_with("OAuth "), "{auth}");
        assert!(auth.contains("oauth_token=\"test-access-token\""), "{auth}");
        assert!(auth.contains("oauth_signature=\""), "{auth}");
    }

    fn yesterday() -> Window {
        use chrono::TimeZone;
        Window {
            start: Utc.with_ymd_and_hms(2026, 9, 20, 0, 0, 0).unwrap(),
            end: Utc.with_ymd_and_hms(2026, 9, 21, 0, 0, 0).unwrap(),
        }
    }

    fn replies_with_body(status: u16, body: &str) -> Result<Vec<Reply>, ApiError> {
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(httpmock::Method::GET)
                .path("/2/users/42/tweets");
            then.status(status).body(body);
        });
        api_client(&server).replies("42", &yesterday())
    }

    fn body_with_one_reply(overrides: serde_json::Value) -> String {
        let mut tweet = serde_json::json!({
            "id": "1",
            "created_at": "2026-09-20T10:00:00.000Z",
            "text": "a",
            "public_metrics": {"impression_count": 5, "like_count": 0},
            "non_public_metrics": {"user_profile_clicks": 4},
            "referenced_tweets": [{"type": "replied_to", "id": "100"}],
            "in_reply_to_user_id": "9"
        });
        for (field, value) in overrides.as_object().unwrap() {
            if value.is_null() {
                tweet.as_object_mut().unwrap().remove(field);
            } else {
                tweet[field] = value.clone();
            }
        }
        serde_json::json!({
            "data": [tweet],
            "includes": {"users": [{"id": "9", "username": "bob"}]}
        })
        .to_string()
    }

    #[test]
    fn replies_keeps_only_replied_to_tweets_and_resolves_username() {
        let body = r#"{"data":[
            {"id":"1","created_at":"2026-09-20T10:00:00.000Z","text":"a","public_metrics":{"impression_count":5,"like_count":1},"non_public_metrics":{"user_profile_clicks":4},"referenced_tweets":[{"type":"replied_to","id":"100"}],"in_reply_to_user_id":"9"},
            {"id":"2","created_at":"2026-09-20T11:00:00.000Z","text":"plain","public_metrics":{"impression_count":6,"like_count":2},"non_public_metrics":{"user_profile_clicks":4}},
            {"id":"3","created_at":"2026-09-20T12:00:00.000Z","text":"quote","public_metrics":{"impression_count":7,"like_count":3},"non_public_metrics":{"user_profile_clicks":4},"referenced_tweets":[{"type":"quoted","id":"101"}]},
            {"id":"4","created_at":"2026-09-20T13:00:00.000Z","text":"b","public_metrics":{"impression_count":8,"like_count":4},"non_public_metrics":{"user_profile_clicks":4},"referenced_tweets":[{"type":"quoted","id":"101"},{"type":"replied_to","id":"102"}],"in_reply_to_user_id":"10"}
        ],"includes":{"users":[{"id":"9","username":"bob"},{"id":"10","username":"carol"}]}}"#;

        let replies = replies_with_body(200, body).unwrap();

        assert_eq!(replies.len(), 2);
        assert_eq!(replies[0].id, "1");
        assert_eq!(replies[0].to_username, "bob");
        assert_eq!(replies[0].impressions, 5);
        assert_eq!(replies[1].id, "4");
        assert_eq!(replies[1].to_username, "carol");
    }

    #[test]
    fn replies_returns_empty_when_response_has_no_data() {
        let replies = replies_with_body(200, r#"{"meta":{"result_count":0}}"#).unwrap();

        assert!(replies.is_empty());
    }

    #[test]
    fn replies_maps_401_to_needs_reconnect() {
        let err = replies_with_body(401, "unauthorized").unwrap_err();

        assert!(matches!(err, ApiError::NeedsReconnect), "{err:?}");
    }

    #[test]
    fn replies_maps_403_to_needs_reconnect() {
        let err = replies_with_body(403, "forbidden").unwrap_err();

        assert!(matches!(err, ApiError::NeedsReconnect), "{err:?}");
    }

    #[test]
    fn replies_maps_500_to_http_error() {
        let err = replies_with_body(500, "boom").unwrap_err();

        assert!(matches!(err, ApiError::Http(_)), "{err:?}");
    }

    #[test]
    fn replies_maps_malformed_body_to_protocol_error() {
        let err = replies_with_body(200, "not json").unwrap_err();

        assert!(matches!(err, ApiError::Protocol(_)), "{err:?}");
    }

    #[test]
    fn replies_maps_reply_missing_impressions_to_protocol_error() {
        let body = body_with_one_reply(serde_json::json!({"public_metrics": {"like_count": 0}}));

        let err = replies_with_body(200, &body).unwrap_err();

        assert!(matches!(err, ApiError::Protocol(_)), "{err:?}");
    }

    #[test]
    fn replies_maps_reply_missing_likes_to_protocol_error() {
        let body =
            body_with_one_reply(serde_json::json!({"public_metrics": {"impression_count": 5}}));

        let err = replies_with_body(200, &body).unwrap_err();

        assert!(matches!(err, ApiError::Protocol(_)), "{err:?}");
    }

    #[test]
    fn replies_maps_reply_missing_profile_clicks_to_protocol_error() {
        let body = body_with_one_reply(serde_json::json!({"non_public_metrics": {}}));

        let err = replies_with_body(200, &body).unwrap_err();

        assert!(matches!(err, ApiError::Protocol(_)), "{err:?}");
    }

    #[test]
    fn replies_maps_reply_missing_non_public_metrics_to_protocol_error() {
        let body = body_with_one_reply(serde_json::json!({"non_public_metrics": null}));

        let err = replies_with_body(200, &body).unwrap_err();

        assert!(matches!(err, ApiError::Protocol(_)), "{err:?}");
    }
}
