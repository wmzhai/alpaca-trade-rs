use std::sync::Arc;

use crate::account::Account;
use crate::client::Inner;
use crate::error::Error;
use crate::transport::endpoint::Endpoint;

#[derive(Debug, Clone)]
pub struct AccountClient {
    inner: Arc<Inner>,
}

impl AccountClient {
    pub(crate) fn new(inner: Arc<Inner>) -> Self {
        Self { inner }
    }

    pub async fn get(&self) -> Result<Account, Error> {
        self.inner
            .http
            .get_json(
                &self.inner.base_url,
                Endpoint::Account,
                &self.inner.auth,
                vec![],
            )
            .await
    }
}
