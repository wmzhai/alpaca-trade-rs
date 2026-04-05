use std::sync::Arc;

use crate::client::Inner;

#[derive(Debug, Clone)]
pub struct ClockClient {
    #[allow(dead_code)]
    inner: Arc<Inner>,
}

impl ClockClient {
    pub(crate) fn new(inner: Arc<Inner>) -> Self {
        Self { inner }
    }

    #[allow(dead_code)]
    pub(crate) fn inner(&self) -> &Arc<Inner> {
        &self.inner
    }
}
