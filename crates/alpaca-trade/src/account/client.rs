use std::sync::Arc;
use std::{fmt, fmt::Debug};

use crate::account::Account;
use crate::client::Inner;
use crate::error::Error;
use crate::transport::endpoint::Endpoint;

#[derive(Clone)]
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
                Endpoint::account_get(),
                &self.inner.auth,
                vec![],
            )
            .await
    }
}

impl Debug for AccountClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let _ = &self.inner;
        f.debug_struct("AccountClient").finish_non_exhaustive()
    }
}
