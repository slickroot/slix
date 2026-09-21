use std::borrow::Cow;
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
        let mut params = HashMap::new();
        params.insert("oauth_callback", Cow::Borrowed("oob"));
        let authorization = oauth1::authorize(
            "POST",
            &self.request_token_endpoint,
            &self.consumer,
            None,
            Some(params),
        );
        let response = self
            .client
            .post(&self.request_token_endpoint)
            .header(AUTHORIZATION, authorization)
            .send()
            .map_err(OAuthError::Http)?
            .error_for_status()
            .map_err(OAuthError::Http)?;
        let params = form_params(&response.text().map_err(OAuthError::Http)?);
        let token = params
            .get("oauth_token")
            .ok_or_else(|| OAuthError::Protocol("missing oauth_token".into()))?
            .clone();
        let secret = params
            .get("oauth_token_secret")
            .ok_or_else(|| OAuthError::Protocol("missing oauth_token_secret".into()))?
            .clone();
        self.request_token = Some(oauth1::Token::new(token.clone(), secret));
        Ok(format!("{}?oauth_token={}", self.authorize_endpoint, token))
    }

    pub fn exchange(&self, pin: &str) -> Result<(String, String), OAuthError> {
        let request_token = self.request_token.as_ref().ok_or_else(|| {
            OAuthError::Protocol("no request token; call authorize_url first".into())
        })?;
        let mut params = HashMap::new();
        params.insert("oauth_verifier", Cow::Borrowed(pin));
        let authorization = oauth1::authorize(
            "POST",
            &self.access_token_endpoint,
            &self.consumer,
            Some(request_token),
            Some(params),
        );
        let response = self
            .client
            .post(&self.access_token_endpoint)
            .header(AUTHORIZATION, authorization)
            .send()
            .map_err(OAuthError::Http)?
            .error_for_status()
            .map_err(OAuthError::Http)?;
        let params = form_params(&response.text().map_err(OAuthError::Http)?);
        let token = params
            .get("oauth_token")
            .ok_or_else(|| OAuthError::Protocol("missing oauth_token".into()))?
            .clone();
        let secret = params
            .get("oauth_token_secret")
            .ok_or_else(|| OAuthError::Protocol("missing oauth_token_secret".into()))?
            .clone();
        Ok((token, secret))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use httpmock::MockServer;

    fn pin_flow(server: &MockServer) -> PinFlow {
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

        let mut flow = pin_flow(&server);
        let url = flow.authorize_url().unwrap();

        let base = server.base_url();
        assert!(
            url.starts_with(&format!("{base}/oauth/authorize?")),
            "{url}"
        );
        assert!(url.contains("oauth_token=RTTOK"), "{url}");
    }

    #[test]
    fn authorize_url_sends_signed_request_with_callback_and_fresh_nonce() {
        let server = MockServer::start();
        let seen_headers = std::sync::Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
        let seen_headers_for_mock = std::sync::Arc::clone(&seen_headers);
        server.mock(|when, then| {
            when.method(httpmock::Method::POST)
                .path("/oauth/request_token");
            then.respond_with(move |req: &httpmock::HttpMockRequest| {
                let auth = req
                    .headers()
                    .get("authorization")
                    .map(|value| value.to_str().unwrap().to_string())
                    .unwrap_or_default();
                seen_headers_for_mock.lock().unwrap().push(auth);
                httpmock::HttpMockResponse::builder()
                    .status(200)
                    .body("oauth_token=RTTOK&oauth_token_secret=RTSEC")
                    .build()
            });
        });

        let mut flow = pin_flow(&server);
        flow.authorize_url().unwrap();
        flow.authorize_url().unwrap();

        let headers = seen_headers.lock().unwrap();
        assert_eq!(headers.len(), 2);
        for header in headers.iter() {
            assert!(header.starts_with("OAuth "), "{header}");
            assert!(header.contains("oauth_callback=\"oob\""), "{header}");
        }
        assert_ne!(headers[0], headers[1], "each call must use a fresh nonce");
    }

    #[test]
    fn exchange_trades_pin_for_access_token_pair() {
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(httpmock::Method::POST)
                .path("/oauth/request_token");
            then.status(200)
                .body("oauth_token=RTTOK&oauth_token_secret=RTSEC");
        });

        let seen_auth = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
        let seen_auth_for_mock = std::sync::Arc::clone(&seen_auth);
        server.mock(|when, then| {
            when.method(httpmock::Method::POST)
                .path("/oauth/access_token");
            then.respond_with(move |req: &httpmock::HttpMockRequest| {
                let auth = req
                    .headers()
                    .get("authorization")
                    .map(|value| value.to_str().unwrap().to_string())
                    .unwrap_or_default();
                *seen_auth_for_mock.lock().unwrap() = auth;
                httpmock::HttpMockResponse::builder()
                    .status(200)
                    .body("oauth_token=ATTOK&oauth_token_secret=ATSEC")
                    .build()
            });
        });

        let mut flow = pin_flow(&server);
        flow.authorize_url().unwrap();
        let (token, secret) = flow.exchange("123456").unwrap();

        assert_eq!(token, "ATTOK");
        assert_eq!(secret, "ATSEC");

        let auth = seen_auth.lock().unwrap();
        assert!(auth.contains("oauth_token=\"RTTOK\""), "{auth}");
        assert!(auth.contains("oauth_verifier=\"123456\""), "{auth}");
    }

    #[test]
    fn exchange_maps_http_and_protocol_failures_to_errors() {
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(httpmock::Method::POST)
                .path("/oauth/request_token");
            then.status(200)
                .body("oauth_token=RTTOK&oauth_token_secret=RTSEC");
        });
        server.mock(|when, then| {
            when.method(httpmock::Method::POST)
                .path("/oauth/access_token");
            then.status(401).body("denied");
        });

        let mut flow = pin_flow(&server);
        flow.authorize_url().unwrap();
        let err = flow.exchange("123456").unwrap_err();
        assert!(matches!(err, OAuthError::Http(_)), "{err:?}");

        server.reset();
        server.mock(|when, then| {
            when.method(httpmock::Method::POST)
                .path("/oauth/request_token");
            then.status(200)
                .body("oauth_token=RTTOK&oauth_token_secret=RTSEC");
        });
        server.mock(|when, then| {
            when.method(httpmock::Method::POST)
                .path("/oauth/access_token");
            then.status(200).body("not-a-token-pair");
        });

        let mut flow = pin_flow(&server);
        flow.authorize_url().unwrap();
        let err = flow.exchange("123456").unwrap_err();
        assert!(matches!(err, OAuthError::Protocol(_)), "{err:?}");
    }
}
