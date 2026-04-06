mod support;

use alpaca_trade::{Client, assets::ListRequest};

#[tokio::test]
async fn assets_live_reads_paper_assets_list_and_single_asset() {
    let Some(credentials) = support::trade_credentials() else {
        eprintln!(
            "skipping live assets test: missing ALPACA_TRADE_API_KEY / ALPACA_TRADE_SECRET_KEY or APCA_API_KEY_ID / APCA_API_SECRET_KEY"
        );
        return;
    };

    let client = Client::builder()
        .api_key(credentials.api_key)
        .secret_key(credentials.secret_key)
        .paper()
        .build()
        .expect("client should build");

    let assets = client
        .assets()
        .list(ListRequest {
            status: Some("active".into()),
            asset_class: Some("us_equity".into()),
            exchange: Some("NASDAQ".into()),
            attributes: Some(vec!["has_options".into()]),
        })
        .await
        .expect("live assets list request should succeed");

    assert!(!assets.is_empty());
    assert!(!assets[0].symbol.is_empty());

    let asset = client
        .assets()
        .get("AAPL")
        .await
        .expect("live asset get request should succeed");

    assert_eq!(asset.symbol, "AAPL");
    assert!(asset.margin_requirement_short.is_some());
}
