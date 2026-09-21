use std::fmt;

use oauth1::Token;
use reqwest::blocking::Client;
use serde::Deserialize;

pub struct XApiClient {
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
            Self::NeedsReconnect => write!(f, "stored tokens no longer accepted; reconnect required"),
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

#[derive(Deserialize)]
struct UserMe {
    data: UserMeData,
}

#[derive(Deserialize)]
struct UserMeData {
    username: String,
}

impl XApiClient {
    pub fn new(
        consumer_key: &str,
        consumer_secret: &str,
        access_token: &str,
        access_token_secret: &str,
    ) -> Self {
        Self::for_endpoint(
            "https://api.x.com/2/",
            consumer_key,
            consumer_secret,
            access_token,
            access_token_secret,
            Client::new(),
        )
    }

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
            endpoint: format!("{base}/users/me"),
            consumer: Token::new(consumer_key.to_string(), consumer_secret.to_string()),
            access_token: Token::new(access_token.to_string(), access_token_secret.to_string()),
            client,
        }
    }

    pub fn users_me(&self) -> Result<String, ApiError> {
        let response = self
            .client
            .get(&self.endpoint)
            .send()
            .map_err(ApiError::Http)?;
        let body = response.text().map_err(ApiError::Http)?;
        let me: UserMe =
            serde_json::from_str(&body).map_err(|e| ApiError::Protocol(e.to_string()))?;
        Ok(format!("@{}", me.data.username))
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
    fn users_me_returns_handle_with_at_prefix() {
        let server = MockServer::start();
        let username = "slickroot";
        server.mock(|when, then| {
            when.method(httpmock::Method::GET).path("/2/users/me");
            then.status(200).body(format!(
                r#"{{"data":{{"id":"1","name":"Slick Root","username":"{username}"}}}}"#
            ));
        });

        let client = api_client(&server);
        let handle = client.users_me().unwrap();

        assert_eq!(handle, format!("@{username}"));
    }
}