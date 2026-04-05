use std::sync::Arc;
use std::{fmt, fmt::Debug};

use crate::assets::{Asset, ListRequest};
use crate::client::Inner;
use crate::error::Error;
use crate::transport::endpoint::Endpoint;
use crate::transport::request::RequestParts;

#[derive(Clone)]
pub struct AssetsClient {
    inner: Arc<Inner>,
}

impl AssetsClient {
    pub(crate) fn new(inner: Arc<Inner>) -> Self {
        Self { inner }
    }

    pub async fn list(&self, request: ListRequest) -> Result<Vec<Asset>, Error> {
        self.inner
            .http
            .send_json(
                &self.inner.base_url,
                &Endpoint::assets_list(),
                &self.inner.auth,
                RequestParts::with_query(request.to_query()),
            )
            .await
    }

    pub async fn get(&self, symbol_or_asset_id: &str) -> Result<Asset, Error> {
        let endpoint = Endpoint::asset_get(symbol_or_asset_id)?;

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

impl Debug for AssetsClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AssetsClient").finish_non_exhaustive()
    }
}
