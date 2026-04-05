use alpaca_trade::Client;
use alpaca_trade::assets::ListRequest;
#[path = "support/http_server.rs"]
mod http_server;

use http_server::TestServer;

fn assets_json() -> &'static str {
    r#"[{"id":"904837e3-3b76-47ec-b432-046db621571b","class":"us_equity","exchange":"NASDAQ","symbol":"AAPL","name":"Apple Inc. Common Stock","status":"active","tradable":true,"marginable":true,"shortable":true,"easy_to_borrow":true,"fractionable":true}]"#
}

#[tokio::test]
async fn assets_list_hits_official_path_query_and_sends_auth_headers() {
    let server = TestServer::spawn(vec![format!(
        "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
        assets_json().len(),
        assets_json()
    )]);

    let assets = Client::builder()
        .api_key("key")
        .secret_key("secret")
        .base_url(server.base_url())
        .build()
        .expect("client should build")
        .assets()
        .list(ListRequest {
            status: Some("active".to_owned()),
            asset_class: Some("us_equity".to_owned()),
            exchange: Some("NASDAQ".to_owned()),
            attributes: Some(vec!["has_options".to_owned()]),
        })
        .await
        .expect("assets request should succeed");

    assert_eq!(assets.len(), 1);
    assert_eq!(assets[0].symbol, "AAPL");

    let request = server.into_single_request();
    assert_eq!(
        request.request_line,
        "GET /v2/assets?status=active&asset_class=us_equity&exchange=NASDAQ&attributes=has_options HTTP/1.1"
    );
    assert!(request.body.is_empty());
    assert_eq!(
        request.headers.get("apca-api-key-id"),
        Some(&"key".to_owned())
    );
    assert_eq!(
        request.headers.get("apca-api-secret-key"),
        Some(&"secret".to_owned())
    );
}
