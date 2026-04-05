use std::sync::Arc;
use std::time::Duration;
use std::{fmt, fmt::Debug};

use crate::account::AccountClient;
use crate::auth::Auth;
use crate::calendar::CalendarClient;
use crate::clock::ClockClient;
use crate::error::Error;
use crate::transport::http::HttpClient;

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);
const PAPER_BASE_URL: &str = "https://paper-api.alpaca.markets";
const LIVE_BASE_URL: &str = "https://api.alpaca.markets";

#[derive(Clone)]
pub struct Client {
    inner: Arc<Inner>,
}

#[derive(Clone)]
pub(crate) struct Inner {
    pub(crate) auth: Auth,
    pub(crate) base_url: String,
    #[allow(dead_code)]
    pub(crate) timeout: Duration,
    pub(crate) http: HttpClient,
}

#[derive(Debug, Clone)]
enum Environment {
    Paper,
    Live,
}

#[derive(Debug)]
pub struct ClientBuilder {
    api_key: Option<String>,
    secret_key: Option<String>,
    environment: Environment,
    base_url: Option<String>,
    timeout: Duration,
}

impl Default for ClientBuilder {
    fn default() -> Self {
        Self {
            api_key: None,
            secret_key: None,
            environment: Environment::Paper,
            base_url: None,
            timeout: DEFAULT_TIMEOUT,
        }
    }
}

impl Client {
    pub fn builder() -> ClientBuilder {
        ClientBuilder::default()
    }

    pub fn account(&self) -> AccountClient {
        AccountClient::new(Arc::clone(&self.inner))
    }

    pub fn calendar(&self) -> CalendarClient {
        CalendarClient::new(Arc::clone(&self.inner))
    }

    pub fn clock(&self) -> ClockClient {
        ClockClient::new(Arc::clone(&self.inner))
    }
}

impl Debug for Client {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Client")
            .field("base_url", &self.inner.base_url)
            .finish_non_exhaustive()
    }
}

impl ClientBuilder {
    pub fn api_key(mut self, api_key: impl Into<String>) -> Self {
        self.api_key = Some(api_key.into());
        self
    }

    pub fn secret_key(mut self, secret_key: impl Into<String>) -> Self {
        self.secret_key = Some(secret_key.into());
        self
    }

    pub fn paper(mut self) -> Self {
        self.environment = Environment::Paper;
        self
    }

    pub fn live(mut self) -> Self {
        self.environment = Environment::Live;
        self
    }

    pub fn base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = Some(base_url.into());
        self
    }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn build(self) -> Result<Client, Error> {
        let auth = Auth::new(self.api_key, self.secret_key)?;
        let http = HttpClient::new(self.timeout)?;
        let base_url = self.base_url.unwrap_or_else(|| match self.environment {
            Environment::Paper => PAPER_BASE_URL.to_owned(),
            Environment::Live => LIVE_BASE_URL.to_owned(),
        });
        reqwest::Url::parse(&base_url)
            .map_err(|error| Error::InvalidConfiguration(format!("invalid base_url: {error}")))?;

        Ok(Client {
            inner: Arc::new(Inner {
                auth,
                base_url,
                timeout: self.timeout,
                http,
            }),
        })
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{Client, DEFAULT_TIMEOUT, LIVE_BASE_URL, PAPER_BASE_URL};

    #[test]
    fn builder_uses_paper_base_url_by_default() {
        let client = Client::builder()
            .api_key("key")
            .secret_key("secret")
            .build()
            .expect("client should build");

        assert_eq!(client.inner.base_url, PAPER_BASE_URL);
        assert_eq!(client.inner.timeout, DEFAULT_TIMEOUT);
    }

    #[test]
    fn builder_switches_to_live_base_url() {
        let client = Client::builder()
            .api_key("key")
            .secret_key("secret")
            .live()
            .build()
            .expect("client should build");

        assert_eq!(client.inner.base_url, LIVE_BASE_URL);
    }

    #[test]
    fn builder_allows_paper_override() {
        let client = Client::builder()
            .api_key("key")
            .secret_key("secret")
            .live()
            .paper()
            .build()
            .expect("client should build");

        assert_eq!(client.inner.base_url, PAPER_BASE_URL);
    }

    #[test]
    fn builder_allows_base_url_and_timeout_override() {
        let client = Client::builder()
            .api_key("key")
            .secret_key("secret")
            .base_url("http://localhost:4010")
            .timeout(Duration::from_secs(5))
            .build()
            .expect("client should build");

        assert_eq!(client.inner.base_url, "http://localhost:4010");
        assert_eq!(client.inner.timeout, Duration::from_secs(5));
    }
}
