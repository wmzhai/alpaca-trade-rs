use reqwest::Method;

use crate::common::validate::required_path_segment;
use crate::error::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Endpoint {
    name: &'static str,
    method: Method,
    path: String,
    requires_auth: bool,
}

impl Endpoint {
    pub(crate) fn new(
        name: &'static str,
        method: Method,
        path: impl Into<String>,
        requires_auth: bool,
    ) -> Self {
        Self {
            name,
            method,
            path: path.into(),
            requires_auth,
        }
    }

    pub(crate) fn account_get() -> Self {
        Self::new("account.get", Method::GET, "/v2/account", true)
    }

    pub(crate) fn clock_get() -> Self {
        Self::new("clock.get", Method::GET, "/v2/clock", true)
    }

    pub(crate) fn calendar_list() -> Self {
        Self::new("calendar.list", Method::GET, "/v2/calendar", true)
    }

    pub(crate) fn assets_list() -> Self {
        Self::new("assets.list", Method::GET, "/v2/assets", true)
    }

    pub(crate) fn options_contracts_list() -> Self {
        Self::new(
            "options_contracts.list",
            Method::GET,
            "/v2/options/contracts",
            true,
        )
    }

    pub(crate) fn orders_list() -> Self {
        Self::new("orders.list", Method::GET, "/v2/orders", true)
    }

    pub(crate) fn orders_create() -> Self {
        Self::new("orders.create", Method::POST, "/v2/orders", true)
    }

    pub(crate) fn orders_cancel_all() -> Self {
        Self::new("orders.cancel_all", Method::DELETE, "/v2/orders", true)
    }

    #[allow(dead_code)]
    pub(crate) fn asset_get(symbol_or_asset_id: &str) -> Result<Self, Error> {
        let symbol_or_asset_id = required_path_segment("symbol_or_asset_id", symbol_or_asset_id)?;

        Ok(Self::new(
            "assets.get",
            Method::GET,
            format!("/v2/assets/{symbol_or_asset_id}"),
            true,
        ))
    }

    pub(crate) fn option_contract_get(symbol_or_id: &str) -> Result<Self, Error> {
        let symbol_or_id = required_path_segment("symbol_or_id", symbol_or_id)?;

        Ok(Self::new(
            "options_contracts.get",
            Method::GET,
            format!("/v2/options/contracts/{symbol_or_id}"),
            true,
        ))
    }

    pub(crate) fn order_get(order_id: &str) -> Result<Self, Error> {
        let order_id = required_path_segment("order_id", order_id)?;

        Ok(Self::new(
            "orders.get",
            Method::GET,
            format!("/v2/orders/{order_id}"),
            true,
        ))
    }

    pub(crate) fn order_replace(order_id: &str) -> Result<Self, Error> {
        let order_id = required_path_segment("order_id", order_id)?;

        Ok(Self::new(
            "orders.replace",
            Method::PATCH,
            format!("/v2/orders/{order_id}"),
            true,
        ))
    }

    pub(crate) fn order_cancel(order_id: &str) -> Result<Self, Error> {
        let order_id = required_path_segment("order_id", order_id)?;

        Ok(Self::new(
            "orders.cancel",
            Method::DELETE,
            format!("/v2/orders/{order_id}"),
            true,
        ))
    }

    pub(crate) fn order_get_by_client_order_id() -> Self {
        Self::new(
            "orders.get_by_client_order_id",
            Method::GET,
            "/v2/orders:by_client_order_id",
            true,
        )
    }

    pub(crate) fn name(&self) -> &'static str {
        self.name
    }

    pub(crate) fn method(&self) -> Method {
        self.method.clone()
    }

    pub(crate) fn path(&self) -> &str {
        &self.path
    }

    pub(crate) fn requires_auth(&self) -> bool {
        self.requires_auth
    }
}

#[cfg(test)]
mod tests {
    use reqwest::Method;

    use super::Endpoint;
    use crate::error::Error;

    #[test]
    fn assets_list_uses_metadata_backed_request_shape() {
        let endpoint = Endpoint::assets_list();

        assert_eq!(endpoint.name(), "assets.list");
        assert_eq!(endpoint.method(), Method::GET);
        assert_eq!(endpoint.path(), "/v2/assets");
        assert!(endpoint.requires_auth());
    }

    #[test]
    fn options_contracts_list_uses_metadata_backed_request_shape() {
        let endpoint = Endpoint::options_contracts_list();

        assert_eq!(endpoint.name(), "options_contracts.list");
        assert_eq!(endpoint.method(), Method::GET);
        assert_eq!(endpoint.path(), "/v2/options/contracts");
        assert!(endpoint.requires_auth());
    }

    #[test]
    fn orders_list_uses_metadata_backed_request_shape() {
        let endpoint = Endpoint::orders_list();

        assert_eq!(endpoint.name(), "orders.list");
        assert_eq!(endpoint.method(), Method::GET);
        assert_eq!(endpoint.path(), "/v2/orders");
        assert!(endpoint.requires_auth());
    }

    #[test]
    fn orders_write_endpoints_preserve_metadata() {
        let create = Endpoint::orders_create();
        let cancel_all = Endpoint::orders_cancel_all();
        let replace = Endpoint::order_replace("order-id-123").expect("replace should build");
        let cancel = Endpoint::order_cancel("order-id-123").expect("cancel should build");

        assert_eq!(create.name(), "orders.create");
        assert_eq!(create.method(), Method::POST);
        assert_eq!(create.path(), "/v2/orders");
        assert!(create.requires_auth());

        assert_eq!(cancel_all.name(), "orders.cancel_all");
        assert_eq!(cancel_all.method(), Method::DELETE);
        assert_eq!(cancel_all.path(), "/v2/orders");
        assert!(cancel_all.requires_auth());

        assert_eq!(replace.name(), "orders.replace");
        assert_eq!(replace.method(), Method::PATCH);
        assert_eq!(replace.path(), "/v2/orders/order-id-123");
        assert!(replace.requires_auth());

        assert_eq!(cancel.name(), "orders.cancel");
        assert_eq!(cancel.method(), Method::DELETE);
        assert_eq!(cancel.path(), "/v2/orders/order-id-123");
        assert!(cancel.requires_auth());
    }

    #[test]
    fn asset_get_uses_metadata_backed_request_shape() {
        let endpoint = Endpoint::asset_get("AAPL").expect("asset endpoint should build");

        assert_eq!(endpoint.name(), "assets.get");
        assert_eq!(endpoint.method(), Method::GET);
        assert_eq!(endpoint.path(), "/v2/assets/AAPL");
        assert!(endpoint.requires_auth());
    }

    #[test]
    fn option_contract_get_uses_metadata_backed_request_shape() {
        let endpoint = Endpoint::option_contract_get("AAPL250620C00100000")
            .expect("options contract endpoint should build");

        assert_eq!(endpoint.name(), "options_contracts.get");
        assert_eq!(endpoint.method(), Method::GET);
        assert_eq!(endpoint.path(), "/v2/options/contracts/AAPL250620C00100000");
        assert!(endpoint.requires_auth());
    }

    #[test]
    fn order_get_uses_metadata_backed_request_shape() {
        let endpoint = Endpoint::order_get("order-id-123").expect("order endpoint should build");

        assert_eq!(endpoint.name(), "orders.get");
        assert_eq!(endpoint.method(), Method::GET);
        assert_eq!(endpoint.path(), "/v2/orders/order-id-123");
        assert!(endpoint.requires_auth());
    }

    #[test]
    fn order_get_by_client_order_id_uses_alias_endpoint_shape() {
        let endpoint = Endpoint::order_get_by_client_order_id();

        assert_eq!(endpoint.name(), "orders.get_by_client_order_id");
        assert_eq!(endpoint.method(), Method::GET);
        assert_eq!(endpoint.path(), "/v2/orders:by_client_order_id");
        assert!(endpoint.requires_auth());
    }

    #[test]
    fn asset_get_rejects_reserved_url_characters_in_path_segments() {
        for value in ["AAPL/US", "AAPL?draft=true", "AAPL#fragment"] {
            let error =
                Endpoint::asset_get(value).expect_err("reserved URL characters should fail");

            match error {
                Error::InvalidRequest(message) => {
                    assert!(message.contains("symbol_or_asset_id"));
                }
                other => panic!("expected invalid request error, got {other:?}"),
            }
        }
    }

    #[test]
    fn option_contract_get_rejects_reserved_url_characters_in_path_segments() {
        for value in ["/", "%2F", "AAPL/US", "AAPL?draft=true", "AAPL#fragment"] {
            let error = Endpoint::option_contract_get(value)
                .expect_err("reserved URL characters should fail");

            match error {
                Error::InvalidRequest(message) => {
                    assert!(message.contains("symbol_or_id"));
                    assert!(message.contains("reserved path characters"));
                }
                other => panic!("expected invalid request error, got {other:?}"),
            }
        }
    }

    #[test]
    fn order_identifier_helpers_reject_reserved_chars_and_whitespace() {
        for value in [
            "/",
            "%2F",
            "order/id",
            "order?id=1",
            " order-id ",
            " order-id",
            "order-id ",
        ] {
            let error =
                Endpoint::order_get(value).expect_err("invalid order identifiers should fail");

            match error {
                Error::InvalidRequest(message) => {
                    assert!(message.contains("order_id"));
                }
                other => panic!("expected invalid request error, got {other:?}"),
            }
        }
    }

    #[test]
    fn asset_get_rejects_leading_or_trailing_whitespace_in_path_segments() {
        for value in [" AAPL", "AAPL ", " AAPL "] {
            let error = Endpoint::asset_get(value).expect_err("surrounding whitespace should fail");

            match error {
                Error::InvalidRequest(message) => {
                    assert!(message.contains("symbol_or_asset_id"));
                    assert!(message.contains("leading or trailing whitespace"));
                }
                other => panic!("expected invalid request error, got {other:?}"),
            }
        }
    }

    #[test]
    fn option_contract_get_rejects_leading_or_trailing_whitespace_in_path_segments() {
        for value in [
            " AAPL250620C00100000",
            "AAPL250620C00100000 ",
            " AAPL250620C00100000 ",
        ] {
            let error = Endpoint::option_contract_get(value)
                .expect_err("surrounding whitespace should fail");

            match error {
                Error::InvalidRequest(message) => {
                    assert!(message.contains("symbol_or_id"));
                    assert!(message.contains("leading or trailing whitespace"));
                }
                other => panic!("expected invalid request error, got {other:?}"),
            }
        }
    }

    #[test]
    fn static_endpoint_helpers_preserve_metadata() {
        let account = Endpoint::account_get();
        let clock = Endpoint::clock_get();
        let calendar = Endpoint::calendar_list();
        let assets = Endpoint::assets_list();
        let options_contracts = Endpoint::options_contracts_list();
        let orders = Endpoint::orders_list();

        assert_eq!(account.name(), "account.get");
        assert_eq!(account.method(), Method::GET);
        assert_eq!(account.path(), "/v2/account");
        assert!(account.requires_auth());

        assert_eq!(clock.name(), "clock.get");
        assert_eq!(clock.method(), Method::GET);
        assert_eq!(clock.path(), "/v2/clock");
        assert!(clock.requires_auth());

        assert_eq!(calendar.name(), "calendar.list");
        assert_eq!(calendar.method(), Method::GET);
        assert_eq!(calendar.path(), "/v2/calendar");
        assert!(calendar.requires_auth());

        assert_eq!(assets.name(), "assets.list");
        assert_eq!(assets.method(), Method::GET);
        assert_eq!(assets.path(), "/v2/assets");
        assert!(assets.requires_auth());

        assert_eq!(options_contracts.name(), "options_contracts.list");
        assert_eq!(options_contracts.method(), Method::GET);
        assert_eq!(options_contracts.path(), "/v2/options/contracts");
        assert!(options_contracts.requires_auth());

        assert_eq!(orders.name(), "orders.list");
        assert_eq!(orders.method(), Method::GET);
        assert_eq!(orders.path(), "/v2/orders");
        assert!(orders.requires_auth());
    }
}
