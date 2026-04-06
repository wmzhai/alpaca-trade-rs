use std::sync::Arc;
use std::{fmt, fmt::Debug};

use crate::client::Inner;
use crate::error::Error;
use crate::options_contracts::{ListRequest, ListResponse, OptionContract};
use crate::transport::endpoint::Endpoint;
use crate::transport::request::RequestParts;

#[derive(Clone)]
pub struct OptionContractsClient {
    pub(crate) inner: Arc<Inner>,
}

impl OptionContractsClient {
    pub(crate) fn new(inner: Arc<Inner>) -> Self {
        Self { inner }
    }

    pub async fn list(&self, request: ListRequest) -> Result<ListResponse, Error> {
        self.inner
            .http
            .send_json(
                &self.inner.base_url,
                &Endpoint::options_contracts_list(),
                &self.inner.auth,
                RequestParts::with_query(request.to_query()?),
            )
            .await
    }

    pub async fn get(&self, symbol_or_id: &str) -> Result<OptionContract, Error> {
        let endpoint = Endpoint::option_contract_get(symbol_or_id)?;

        self.inner
            .http
            .send_json(
                &self.inner.base_url,
                &endpoint,
                &self.inner.auth,
                RequestParts::with_query(Vec::new()),
            )
            .await
    }
}

impl Debug for OptionContractsClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let _ = &self.inner;
        f.debug_struct("OptionContractsClient")
            .finish_non_exhaustive()
    }
}
