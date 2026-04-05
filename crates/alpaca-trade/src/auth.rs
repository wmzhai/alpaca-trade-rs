use std::{fmt, fmt::Debug};

use reqwest::header::{HeaderName, HeaderValue};

use crate::error::Error;

const APCA_API_KEY_ID: HeaderName = HeaderName::from_static("apca-api-key-id");
const APCA_API_SECRET_KEY: HeaderName = HeaderName::from_static("apca-api-secret-key");

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct Auth {
    api_key: String,
    secret_key: String,
}

impl Auth {
    pub(crate) fn new(api_key: Option<String>, secret_key: Option<String>) -> Result<Self, Error> {
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

    pub(crate) fn apply(
        &self,
        request: reqwest::RequestBuilder,
    ) -> Result<reqwest::RequestBuilder, Error> {
        let api_key = HeaderValue::from_str(&self.api_key).map_err(|error| {
            Error::InvalidConfiguration(format!("invalid api_key header value: {error}"))
        })?;
        let secret_key = HeaderValue::from_str(&self.secret_key).map_err(|error| {
            Error::InvalidConfiguration(format!("invalid secret_key header value: {error}"))
        })?;

        Ok(request
            .header(APCA_API_KEY_ID.clone(), api_key)
            .header(APCA_API_SECRET_KEY.clone(), secret_key))
    }
}

impl Debug for Auth {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Auth").finish_non_exhaustive()
    }
}
