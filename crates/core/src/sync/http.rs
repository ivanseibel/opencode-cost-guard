//! Injectable HTTP access for Catalog Sync.
//!
//! The fetch path goes through [`HttpClient`] so tests script responses
//! without network access (SYNC-09). The production implementation is
//! [`UreqClient`], configured with a per-request timeout (SYNC-07).

use std::time::Duration;

/// Default per-request timeout (SYNC-07).
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);

/// A completed HTTP response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpResponse {
    pub status: u16,
    pub body: String,
    pub etag: Option<String>,
}

/// HTTP access required by Catalog Sync.
pub trait HttpClient {
    /// GET `url`, sending `If-None-Match` when an ETag is present.
    fn get(&self, url: &str, etag: Option<&str>) -> Result<HttpResponse, HttpError>;
}

/// A request to send, built without performing I/O.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpRequest {
    pub url: String,
    /// Headers in insertion order.
    pub headers: Vec<(String, String)>,
}

/// Build the GET request for `url`, adding `If-None-Match` iff `etag` exists.
pub fn build_request(url: &str, etag: Option<&str>) -> HttpRequest {
    let headers = etag
        .map(|etag| vec![("If-None-Match".to_string(), etag.to_string())])
        .unwrap_or_default();

    HttpRequest {
        url: url.to_string(),
        headers,
    }
}

/// Errors from an HTTP fetch.
#[derive(Debug, thiserror::Error)]
pub enum HttpError {
    #[error("request timed out: {0}")]
    Timeout(String),
    #[error("request failed: {0}")]
    Transport(String),
    #[error("failed to read response body: {0}")]
    Body(String),
}

/// Blocking `ureq` implementation with a fixed timeout.
#[derive(Debug)]
pub struct UreqClient {
    agent: ureq::Agent,
    timeout: Duration,
}

impl UreqClient {
    /// Build a client whose requests time out after `timeout`.
    pub fn new(timeout: Duration) -> Self {
        let agent = ureq::Agent::config_builder()
            .http_status_as_error(false)
            .timeout_global(Some(timeout))
            .build()
            .new_agent();

        Self { agent, timeout }
    }

    /// The timeout applied to every request.
    pub fn timeout(&self) -> Duration {
        self.timeout
    }
}

impl Default for UreqClient {
    fn default() -> Self {
        Self::new(DEFAULT_TIMEOUT)
    }
}

impl HttpClient for UreqClient {
    fn get(&self, url: &str, etag: Option<&str>) -> Result<HttpResponse, HttpError> {
        let request = build_request(url, etag);
        let mut builder = self.agent.get(&request.url);
        for (name, value) in &request.headers {
            builder = builder.header(name.as_str(), value.as_str());
        }

        let mut response = builder.call().map_err(|error| match error {
            ureq::Error::Timeout(timeout) => HttpError::Timeout(timeout.to_string()),
            other => HttpError::Transport(other.to_string()),
        })?;

        let status = response.status().as_u16();
        let etag = response
            .headers()
            .get("etag")
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned);
        let body = response
            .body_mut()
            .read_to_string()
            .map_err(|error| HttpError::Body(error.to_string()))?;

        Ok(HttpResponse { status, body, etag })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct StubClient;

    impl HttpClient for StubClient {
        fn get(&self, _url: &str, etag: Option<&str>) -> Result<HttpResponse, HttpError> {
            Ok(HttpResponse {
                status: 200,
                body: "stub body".to_string(),
                etag: etag.map(str::to_owned),
            })
        }
    }

    #[test]
    fn request_builder_sets_if_none_match_when_etag_is_present() {
        let request = build_request("https://example.com/api.json", Some("\"abc123\""));

        assert_eq!(request.url, "https://example.com/api.json");
        assert_eq!(
            request.headers,
            vec![("If-None-Match".to_string(), "\"abc123\"".to_string())]
        );
    }

    #[test]
    fn request_builder_omits_if_none_match_without_etag() {
        let request = build_request("https://example.com/api.json", None);

        assert_eq!(request.url, "https://example.com/api.json");
        assert!(request.headers.is_empty());
    }

    #[test]
    fn timeout_value_reaches_the_agent_config() {
        let client = UreqClient::default();

        assert_eq!(client.timeout(), Duration::from_secs(10));
        assert_eq!(
            client.agent.config().timeouts().global,
            Some(Duration::from_secs(10))
        );

        let custom = UreqClient::new(Duration::from_secs(3));
        assert_eq!(custom.timeout(), Duration::from_secs(3));
        assert_eq!(
            custom.agent.config().timeouts().global,
            Some(Duration::from_secs(3))
        );
    }

    #[test]
    fn stub_client_is_usable_through_the_trait() {
        let client: &dyn HttpClient = &StubClient;

        let response = client
            .get("https://example.com/api.json", Some("\"etag\""))
            .expect("stub response");

        assert_eq!(response.status, 200);
        assert_eq!(response.body, "stub body");
        assert_eq!(response.etag.as_deref(), Some("\"etag\""));
    }
}
