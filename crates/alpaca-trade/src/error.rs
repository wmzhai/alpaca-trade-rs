use std::error::Error as StdError;
use std::fmt;

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorMeta {
    pub endpoint: String,
    pub method: String,
    pub status: Option<u16>,
    pub request_id: Option<String>,
    pub retry_after: Option<String>,
    pub body: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    InvalidConfiguration(String),
    InvalidRequest(String),
    MissingCredentials,
    Transport {
        message: String,
        meta: Option<ErrorMeta>,
    },
    Timeout {
        message: String,
        meta: Option<ErrorMeta>,
    },
    RateLimited(ErrorMeta),
    HttpStatus(ErrorMeta),
    Deserialize {
        message: String,
        meta: ErrorMeta,
    },
}

impl Error {
    pub fn from_reqwest(error: reqwest::Error) -> Self {
        Self::from_reqwest_with_meta(error, None)
    }

    pub(crate) fn from_reqwest_with_meta(error: reqwest::Error, meta: Option<ErrorMeta>) -> Self {
        if error.is_timeout() {
            return Self::Timeout {
                message: error.to_string(),
                meta,
            };
        }

        if error.is_decode() {
            return Self::Deserialize {
                message: error.to_string(),
                meta: meta.unwrap_or_else(ErrorMeta::unknown),
            };
        }

        if let Some(status) = error.status() {
            let meta = meta
                .map(|meta| meta.with_status(status.as_u16()))
                .unwrap_or_else(|| ErrorMeta::unknown().with_status(status.as_u16()));

            if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                return Self::RateLimited(meta);
            }

            return Self::HttpStatus(meta);
        }

        Self::Transport {
            message: error.to_string(),
            meta,
        }
    }

    pub(crate) fn meta(&self) -> Option<&ErrorMeta> {
        match self {
            Self::Transport {
                meta: Some(meta), ..
            }
            | Self::Timeout {
                meta: Some(meta), ..
            }
            | Self::RateLimited(meta)
            | Self::HttpStatus(meta)
            | Self::Deserialize { meta, .. } => Some(meta),
            _ => None,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidConfiguration(message) => write!(f, "invalid configuration: {message}"),
            Self::InvalidRequest(message) => write!(f, "invalid request: {message}"),
            Self::MissingCredentials => write!(f, "missing credentials"),
            Self::Transport { message, meta } => {
                write!(f, "transport error: {message}")?;
                write_meta(f, meta.as_ref())
            }
            Self::Timeout { message, meta } => {
                write!(f, "request timed out: {message}")?;
                write_meta(f, meta.as_ref())
            }
            Self::RateLimited(meta) => {
                write!(f, "rate limited")?;
                write_meta(f, Some(meta))
            }
            Self::HttpStatus(meta) => {
                write!(f, "http status error")?;
                write_meta(f, Some(meta))
            }
            Self::Deserialize { message, meta } => {
                write!(f, "deserialize error: {message}")?;
                write_meta(f, Some(meta))
            }
        }
    }
}

impl StdError for Error {}

impl ErrorMeta {
    pub(crate) fn unknown() -> Self {
        Self {
            endpoint: String::new(),
            method: String::new(),
            status: None,
            request_id: None,
            retry_after: None,
            body: None,
        }
    }

    fn with_status(mut self, status: u16) -> Self {
        self.status = Some(status);
        self
    }
}

fn write_meta(f: &mut fmt::Formatter<'_>, meta: Option<&ErrorMeta>) -> fmt::Result {
    let Some(meta) = meta else {
        return Ok(());
    };

    let mut parts = Vec::new();

    if !meta.endpoint.is_empty() {
        parts.push(format!("endpoint={}", meta.endpoint));
    }

    if !meta.method.is_empty() {
        parts.push(format!("method={}", meta.method));
    }

    if let Some(status) = meta.status {
        parts.push(format!("status={status}"));
    }

    if let Some(request_id) = &meta.request_id {
        parts.push(format!("request_id={request_id}"));
    }

    if let Some(retry_after) = &meta.retry_after {
        parts.push(format!("retry_after={retry_after}"));
    }

    if let Some(body) = &meta.body {
        parts.push(format!("body={body}"));
    }

    if parts.is_empty() {
        return Ok(());
    }

    write!(f, " [{}]", parts.join(", "))
}
