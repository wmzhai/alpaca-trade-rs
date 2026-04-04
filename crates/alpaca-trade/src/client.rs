use std::sync::Arc;

use reqwest::header::HeaderMap;

use crate::account::AccountClient;
use crate::auth::Credentials;
use crate::error::Error;

const PAPER_BASE_URL: &str = "https://paper-api.alpaca.markets";
const LIVE_BASE_URL: &str = "https://api.alpaca.markets";

#[derive(Debug, Clone)]
pub struct Client {
    inner: Arc<Inner>,
}

#[allow(dead_code)]
#[derive(Debug)]
pub(crate) struct Inner {
    pub(crate) base_url: String,
    pub(crate) http_client: reqwest::Client,
}

#[derive(Debug, Default)]
pub struct ClientBuilder {
    api_key: Option<String>,
    secret_key: Option<String>,
    live: bool,
}

impl Client {
    pub fn builder() -> ClientBuilder {
        ClientBuilder::default()
    }

    pub fn account(&self) -> AccountClient {
        AccountClient::new(Arc::clone(&self.inner))
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

    pub fn live(mut self) -> Self {
        self.live = true;
        self
    }

    pub fn build(self) -> Result<Client, Error> {
        let credentials = Credentials::validate(self.api_key, self.secret_key)?;
        let mut headers = HeaderMap::new();
        credentials.apply_headers(&mut headers)?;

        let http_client = reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .map_err(Error::from_reqwest)?;

        let inner = Inner {
            base_url: if self.live {
                LIVE_BASE_URL.to_owned()
            } else {
                PAPER_BASE_URL.to_owned()
            },
            http_client,
        };

        Ok(Client {
            inner: Arc::new(inner),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{Client, LIVE_BASE_URL, PAPER_BASE_URL};

    #[test]
    fn builder_uses_paper_base_url_by_default() {
        let client = Client::builder()
            .api_key("key")
            .secret_key("secret")
            .build()
            .expect("client should build");

        assert_eq!(client.inner.base_url, PAPER_BASE_URL);
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
}
