use std::sync::Arc;
use std::{fmt, fmt::Debug};

use crate::client::Inner;
use crate::error::Error;
use crate::{NoContent, orders::{CancelAllOrderResult, CreateRequest, ListRequest, Order, ReplaceRequest}};

#[derive(Clone)]
pub struct OrdersClient {
    pub(crate) inner: Arc<Inner>,
}

impl OrdersClient {
    pub(crate) fn new(inner: Arc<Inner>) -> Self {
        Self { inner }
    }

    pub async fn list(&self, request: ListRequest) -> Result<Vec<Order>, Error> {
        let _ = &self.inner;
        request.to_query()?;
        Err(orders_not_implemented())
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
        let _ = &self.inner;
        super::request::validate_order_id(order_id)?;
        Err(orders_not_implemented())
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
        let _ = &self.inner;
        super::request::validate_client_order_id(client_order_id)?;
        Err(orders_not_implemented())
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
