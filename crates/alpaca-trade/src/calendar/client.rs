use std::sync::Arc;
use std::{fmt, fmt::Debug};

use crate::client::Inner;
use crate::calendar::{Calendar, ListRequest};
use crate::error::Error;
use crate::transport::endpoint::Endpoint;

#[derive(Clone)]
pub struct CalendarClient {
    inner: Arc<Inner>,
}

impl CalendarClient {
    pub(crate) fn new(inner: Arc<Inner>) -> Self {
        Self { inner }
    }

    pub async fn list(&self, request: ListRequest) -> Result<Vec<Calendar>, Error> {
        self.inner
            .http
            .get_json(
                &self.inner.base_url,
                Endpoint::Calendar,
                &self.inner.auth,
                request.to_query(),
            )
            .await
    }
}

impl Debug for CalendarClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CalendarClient").finish_non_exhaustive()
    }
}
