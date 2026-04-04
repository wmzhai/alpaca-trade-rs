use std::time::Duration;

use serde::de::DeserializeOwned;

use crate::auth::Auth;
use crate::error::Error;
use crate::transport::endpoint::Endpoint;

#[derive(Debug, Clone)]
pub(crate) struct HttpClient {
    client: reqwest::Client,
}

impl HttpClient {
    pub(crate) fn new(timeout: Duration) -> Result<Self, Error> {
        let client = reqwest::Client::builder()
            .timeout(timeout)
            .build()
            .map_err(Error::from_reqwest)?;

        Ok(Self { client })
    }

    pub(crate) async fn get_json<T>(
        &self,
        base_url: &str,
        endpoint: Endpoint,
        auth: &Auth,
        query: Vec<(String, String)>,
    ) -> Result<T, Error>
    where
        T: DeserializeOwned,
    {
        let url = format!("{}{}", base_url.trim_end_matches('/'), endpoint.path());
        let mut request = self.client.get(url).query(&query);

        if endpoint.requires_auth() {
            request = auth.apply(request)?;
        }

        let response = request.send().await.map_err(Error::from_reqwest)?;
        let status = response.status();

        if !status.is_success() {
            let retry_after = response
                .headers()
                .get(reqwest::header::RETRY_AFTER)
                .and_then(|value| value.to_str().ok())
                .map(str::to_owned);
            let body = response.text().await.map_err(Error::from_reqwest)?;
            let body = (!body.is_empty()).then_some(body);

            if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                return Err(Error::RateLimited { retry_after, body });
            }

            return Err(Error::HttpStatus {
                status: status.as_u16(),
                body,
            });
        }

        let body = response.text().await.map_err(Error::from_reqwest)?;
        serde_json::from_str(&body).map_err(|error| Error::Deserialize(error.to_string()))
    }
}
