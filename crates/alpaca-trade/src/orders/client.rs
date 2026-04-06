use std::sync::Arc;
use std::{fmt, fmt::Debug};

use crate::client::Inner;
use crate::error::Error;
use crate::transport::endpoint::Endpoint;
use crate::transport::request::RequestParts;
use crate::{
    NoContent,
    orders::{CancelAllOrderResult, CreateRequest, ListRequest, Order, ReplaceRequest},
};

#[derive(Clone)]
pub struct OrdersClient {
    pub(crate) inner: Arc<Inner>,
}

impl OrdersClient {
    pub(crate) fn new(inner: Arc<Inner>) -> Self {
        Self { inner }
    }

    pub async fn list(&self, request: ListRequest) -> Result<Vec<Order>, Error> {
        self.inner
            .http
            .send_json(
                &self.inner.base_url,
                &Endpoint::orders_list(),
                &self.inner.auth,
                RequestParts::with_query(request.to_query()?),
            )
            .await
    }

    pub async fn create(&self, request: CreateRequest) -> Result<Order, Error> {
        let _ = &self.inner;
        request.to_json()?;
        Err(orders_not_implemented())
    }

    pub async fn cancel_all(&self) -> Result<Vec<CancelAllOrderResult>, Error> {
        let _ = &self.inner;
        Err(orders_not_implemented())
    }

    pub async fn get(&self, order_id: &str) -> Result<Order, Error> {
        let endpoint = Endpoint::order_get(order_id)?;

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

    pub async fn replace(&self, order_id: &str, request: ReplaceRequest) -> Result<Order, Error> {
        let _ = &self.inner;
        super::request::validate_order_id(order_id)?;
        request.to_json()?;
        Err(orders_not_implemented())
    }

    pub async fn cancel(&self, order_id: &str) -> Result<NoContent, Error> {
        let _ = &self.inner;
        super::request::validate_order_id(order_id)?;
        Err(orders_not_implemented())
    }

    pub async fn get_by_client_order_id(&self, client_order_id: &str) -> Result<Order, Error> {
        let client_order_id = super::request::validate_client_order_id(client_order_id)?;

        self.inner
            .http
            .send_json(
                &self.inner.base_url,
                &Endpoint::order_get_by_client_order_id(),
                &self.inner.auth,
                RequestParts::with_query(vec![("client_order_id".to_owned(), client_order_id)]),
            )
            .await
    }
}

impl Debug for OrdersClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let _ = &self.inner;
        f.debug_struct("OrdersClient").finish_non_exhaustive()
    }
}

fn orders_not_implemented() -> Error {
    Error::InvalidConfiguration("orders transport is not implemented yet".to_owned())
}
