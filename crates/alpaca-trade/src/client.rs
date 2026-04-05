use std::sync::Arc;
use std::time::Duration;
use std::{fmt, fmt::Debug};

use crate::account::AccountClient;
use crate::auth::{
    Auth, DEFAULT_API_KEY_ENV, DEFAULT_SECRET_KEY_ENV, load_credentials_from_env,
    load_credentials_from_env_names,
};
use crate::calendar::CalendarClient;
use crate::clock::ClockClient;
use crate::error::Error;
use crate::observer::{NoopObserver, Observer};
use crate::retry::RetryPolicy;
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

pub struct ClientBuilder {
    api_key: Option<String>,
    secret_key: Option<String>,
    credentials_env: Option<CredentialsEnv>,
    environment: Environment,
    base_url: Option<String>,
    timeout: Duration,
    reqwest_client: Option<reqwest::Client>,
    observer: Arc<dyn Observer>,
    retry_policy: RetryPolicy,
    timeout_configured: bool,
    observer_configured: bool,
    retry_policy_configured: bool,
}

#[derive(Debug, Clone)]
struct CredentialsEnv {
    api_key_env: String,
    secret_key_env: String,
}

impl Default for ClientBuilder {
    fn default() -> Self {
        Self {
            api_key: None,
            secret_key: None,
            credentials_env: None,
            environment: Environment::Paper,
            base_url: None,
            timeout: DEFAULT_TIMEOUT,
            reqwest_client: None,
            observer: Arc::new(NoopObserver),
            retry_policy: RetryPolicy::trading_safe(),
            timeout_configured: false,
            observer_configured: false,
            retry_policy_configured: false,
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
        let _ = &self.inner;
        f.debug_struct("Client").finish_non_exhaustive()
    }
}

impl Debug for ClientBuilder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let _ = &self.api_key;
        let _ = &self.secret_key;
        let _ = &self.credentials_env;
        let _ = &self.environment;
        let _ = &self.base_url;
        let _ = &self.timeout;
        let _ = &self.reqwest_client;
        let _ = &self.observer;
        let _ = &self.retry_policy;
        let _ = &self.timeout_configured;
        let _ = &self.observer_configured;
        let _ = &self.retry_policy_configured;
        f.debug_struct("ClientBuilder").finish_non_exhaustive()
    }
}

impl ClientBuilder {
    pub fn api_key(mut self, api_key: impl Into<String>) -> Self {
        self.credentials_env = None;
        self.api_key = Some(api_key.into());
        self
    }

    pub fn secret_key(mut self, secret_key: impl Into<String>) -> Self {
        self.credentials_env = None;
        self.secret_key = Some(secret_key.into());
        self
    }

    pub fn credentials_from_env(mut self) -> Self {
        self.api_key = None;
        self.secret_key = None;
        self.credentials_env = Some(CredentialsEnv {
            api_key_env: DEFAULT_API_KEY_ENV.to_owned(),
            secret_key_env: DEFAULT_SECRET_KEY_ENV.to_owned(),
        });
        self
    }

    pub fn credentials_from_env_names(
        mut self,
        api_key_env: impl Into<String>,
        secret_key_env: impl Into<String>,
    ) -> Self {
        self.api_key = None;
        self.secret_key = None;
        self.credentials_env = Some(CredentialsEnv {
            api_key_env: api_key_env.into(),
            secret_key_env: secret_key_env.into(),
        });
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
        self.timeout_configured = true;
        self
    }

    pub fn reqwest_client(mut self, client: reqwest::Client) -> Self {
        self.reqwest_client = Some(client);
        self
    }

    pub fn observer<O>(mut self, observer: O) -> Self
    where
        O: Observer,
    {
        self.observer = Arc::new(observer);
        self.observer_configured = true;
        self
    }

    pub fn retry_policy(mut self, retry_policy: RetryPolicy) -> Self {
        self.retry_policy = retry_policy;
        self.retry_policy_configured = true;
        self
    }

    pub fn build(self) -> Result<Client, Error> {
        let (api_key, secret_key) = match self.credentials_env.as_ref() {
            Some(credentials_env)
                if credentials_env.api_key_env == DEFAULT_API_KEY_ENV
                    && credentials_env.secret_key_env == DEFAULT_SECRET_KEY_ENV =>
            {
                load_credentials_from_env()?
            }
            Some(credentials_env) => load_credentials_from_env_names(
                &credentials_env.api_key_env,
                &credentials_env.secret_key_env,
            )?,
            None => (self.api_key, self.secret_key),
        };
        let auth = Auth::new(api_key, secret_key)?;
        let base_url = self.base_url.unwrap_or_else(|| match self.environment {
            Environment::Paper => PAPER_BASE_URL.to_owned(),
            Environment::Live => LIVE_BASE_URL.to_owned(),
        });
        reqwest::Url::parse(&base_url)
            .map_err(|error| Error::InvalidConfiguration(format!("invalid base_url: {error}")))?;
        let http = match self.reqwest_client {
            Some(client) => {
                if self.timeout_configured {
                    return Err(Error::InvalidConfiguration(
                        "reqwest_client() cannot be combined with timeout(); configure the timeout on the provided reqwest::Client".to_owned(),
                    ));
                }

                HttpClient::with_client(client, self.retry_policy, self.observer)
            }
            None if !self.observer_configured && !self.retry_policy_configured => {
                HttpClient::new(self.timeout)?
            }
            None => {
                let reqwest_client = reqwest::Client::builder()
                    .timeout(self.timeout)
                    .build()
                    .map_err(Error::from_reqwest)?;
                HttpClient::with_client(reqwest_client, self.retry_policy, self.observer)
            }
        };

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
