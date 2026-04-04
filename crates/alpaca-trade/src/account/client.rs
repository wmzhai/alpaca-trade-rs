use std::sync::Arc;

use crate::client::Inner;

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct AccountClient {
    inner: Arc<Inner>,
}

impl AccountClient {
    pub(crate) fn new(inner: Arc<Inner>) -> Self {
        Self { inner }
    }
}
