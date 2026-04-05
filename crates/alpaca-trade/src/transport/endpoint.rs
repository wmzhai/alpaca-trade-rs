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
    fn new(
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

    #[allow(dead_code)]
    pub(crate) fn name(&self) -> &'static str {
        self.name
    }

    #[allow(dead_code)]
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

    #[test]
    fn asset_get_uses_metadata_backed_request_shape() {
        let endpoint = Endpoint::asset_get("AAPL").expect("asset endpoint should build");

        assert_eq!(endpoint.name(), "assets.get");
        assert_eq!(endpoint.method(), Method::GET);
        assert_eq!(endpoint.path(), "/v2/assets/AAPL");
        assert!(endpoint.requires_auth());
    }

    #[test]
    fn static_endpoint_helpers_preserve_metadata() {
        let account = Endpoint::account_get();
        let clock = Endpoint::clock_get();
        let calendar = Endpoint::calendar_list();

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
    }
}
