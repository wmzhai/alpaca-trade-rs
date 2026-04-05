use std::sync::Arc;
use std::{fmt, fmt::Debug};

use crate::client::Inner;
use crate::clock::Clock;
use crate::error::Error;
use crate::transport::endpoint::Endpoint;

#[derive(Clone)]
pub struct ClockClient {
    _inner: Arc<Inner>,
}

impl ClockClient {
    pub(crate) fn new(inner: Arc<Inner>) -> Self {
        Self { _inner: inner }
    }

    pub async fn get(&self) -> Result<Clock, Error> {
        self._inner
            .http
            .get_json(
                &self._inner.base_url,
                Endpoint::Clock,
                &self._inner.auth,
                vec![],
            )
            .await
    }
}

impl Debug for ClockClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ClockClient").finish_non_exhaustive()
    }
}
