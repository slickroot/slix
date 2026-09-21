use std::collections::HashMap;
use std::fmt;

use reqwest::blocking::Client;
use reqwest::header::AUTHORIZATION;

pub struct PinFlow {
    consumer: oauth1::Token<'static>,
    request_token: Option<oauth1::Token<'static>>,
    request_token_endpoint: String,
    authorize_endpoint: String,
    access_token_endpoint: String,
    client: Client,
}

#[derive(Debug)]
pub enum OAuthError {
    Http(reqwest::Error),
    Protocol(String),
}

impl fmt::Display for OAuthError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Http(e) => write!(f, "oauth http error: {e}"),
            Self::Protocol(msg) => write!(f, "oauth protocol error: {msg}"),
        }
    }
}

impl std::error::Error for OAuthError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Http(e) => Some(e),
            Self::Protocol(_) => None,
        }
    }
}

fn form_params(body: &str) -> HashMap<String, String> {
    body.split('&')
        .filter_map(|pair| {
            pair.split_once('=')
                .map(|(key, value)| (key.to_string(), value.to_string()))
        })
        .collect()
}

impl PinFlow {
    pub fn new(consumer_key: &str, consumer_secret: &str) -> Self {
        Self::for_endpoints(
            "https://api.x.com/oauth",
            consumer_key,
            consumer_secret,
            Client::new(),
        )
    }

    pub fn for_endpoints(
        base_url: &str,
        consumer_key: &str,
        consumer_secret: &str,
        client: Client,
    ) -> Self {
        let base = base_url.trim_end_matches('/');
        Self {
            consumer: oauth1::Token::new(consumer_key.to_string(), consumer_secret.to_string()),
            request_token: None,
            request_token_endpoint: format!("{base}/request_token"),
            authorize_endpoint: format!("{base}/authorize"),
            access_token_endpoint: format!("{base}/access_token"),
            client,
        }
    }

    pub fn authorize_url(&mut self) -> Result<String, OAuthError> {
        let response = self
            .client
            .post(&self.request_token_endpoint)
            .send()
            .map_err(OAuthError::Http)?
            .error_for_status()
            .map_err(OAuthError::Http)?;
        let params = form_params(&response.text().map_err(OAuthError::Http)?);
        let token = params
            .get("oauth_token")
            .ok_or_else(|| OAuthError::Protocol("missing oauth_token".into()))?;
        Ok(format!("{}?oauth_token={}", self.authorize_endpoint, token))
    }

    pub fn exchange(&self, pin: &str) -> Result<(String, String), OAuthError> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use httpmock::MockServer;

    fn flow(server: &MockServer) -> PinFlow {
        PinFlow::for_endpoints(
            &format!("{}/oauth", server.base_url()),
            "test-consumer-key",
            "test-consumer-secret",
            Client::new(),
        )
    }

    #[test]
    fn authorize_url_builds_url_with_request_token_from_response() {
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(httpmock::Method::POST)
                .path("/oauth/request_token");
            then.status(200)
                .body("oauth_token=RTTOK&oauth_token_secret=RTSEC");
        });

        let mut flow = flow(&server);
        let url = flow.authorize_url().unwrap();

        let base = server.base_url();
        assert!(
            url.starts_with(&format!("{base}/oauth/authorize?")),
            "{url}"
        );
        assert!(url.contains("oauth_token=RTTOK"), "{url}");
    }
}
