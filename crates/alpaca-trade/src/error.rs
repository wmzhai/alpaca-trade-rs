use std::error::Error as StdError;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    InvalidConfiguration(String),
    InvalidRequest(String),
    MissingCredentials,
    Transport(String),
    Timeout(String),
    RateLimited {
        retry_after: Option<String>,
        body: Option<String>,
    },
    HttpStatus {
        status: u16,
        body: Option<String>,
    },
    Deserialize(String),
}

impl Error {
    pub fn from_reqwest(error: reqwest::Error) -> Self {
        if error.is_timeout() {
            return Self::Timeout(error.to_string());
        }

        if error.is_decode() {
            return Self::Deserialize(error.to_string());
        }

        if let Some(status) = error.status() {
            if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                return Self::RateLimited {
                    retry_after: None,
                    body: None,
                };
            }

            return Self::HttpStatus {
                status: status.as_u16(),
                body: None,
            };
        }

        Self::Transport(error.to_string())
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidConfiguration(message) => write!(f, "invalid configuration: {message}"),
            Self::InvalidRequest(message) => write!(f, "invalid request: {message}"),
            Self::MissingCredentials => write!(f, "missing credentials"),
            Self::Transport(message) => write!(f, "transport error: {message}"),
            Self::Timeout(message) => write!(f, "request timed out: {message}"),
            Self::RateLimited { retry_after, body } => {
                write!(f, "rate limited")?;
                if let Some(retry_after) = retry_after {
                    write!(f, " (retry after: {retry_after})")?;
                }
                if let Some(body) = body {
                    write!(f, ": {body}")?;
                }
                Ok(())
            }
            Self::HttpStatus { status, body } => {
                write!(f, "http status error: {status}")?;
                if let Some(body) = body {
                    write!(f, ": {body}")?;
                }
                Ok(())
            }
            Self::Deserialize(message) => write!(f, "deserialize error: {message}"),
        }
    }
}

impl StdError for Error {}
