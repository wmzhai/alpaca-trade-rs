use reqwest::header::{HeaderMap, HeaderName, HeaderValue};

use crate::error::Error;

const APCA_API_KEY_ID: HeaderName = HeaderName::from_static("apca-api-key-id");
const APCA_API_SECRET_KEY: HeaderName = HeaderName::from_static("apca-api-secret-key");

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Credentials {
    api_key: String,
    secret_key: String,
}

impl Credentials {
    pub(crate) fn validate(
        api_key: Option<String>,
        secret_key: Option<String>,
    ) -> Result<Self, Error> {
        match (api_key, secret_key) {
            (Some(api_key), Some(secret_key)) => {
                if api_key.trim().is_empty() {
                    return Err(Error::InvalidConfiguration(
                        "api_key must not be empty or whitespace".to_owned(),
                    ));
                }

                if secret_key.trim().is_empty() {
                    return Err(Error::InvalidConfiguration(
                        "secret_key must not be empty or whitespace".to_owned(),
                    ));
                }

                Ok(Self {
                    api_key,
                    secret_key,
                })
            }
            (None, None) => Err(Error::MissingCredentials),
            _ => Err(Error::InvalidConfiguration(
                "api_key and secret_key must be configured together".to_owned(),
            )),
        }
    }

    pub(crate) fn apply_headers(&self, headers: &mut HeaderMap) -> Result<(), Error> {
        let api_key = HeaderValue::from_str(&self.api_key).map_err(|error| {
            Error::InvalidConfiguration(format!("invalid api_key header value: {error}"))
        })?;
        let secret_key = HeaderValue::from_str(&self.secret_key).map_err(|error| {
            Error::InvalidConfiguration(format!("invalid secret_key header value: {error}"))
        })?;

        headers.insert(APCA_API_KEY_ID, api_key);
        headers.insert(APCA_API_SECRET_KEY, secret_key);
        Ok(())
    }
}
