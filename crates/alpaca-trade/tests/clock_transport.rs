use alpaca_trade::Client;
mod support;

use support::TestServer;

fn clock_json() -> &'static str {
    r#"{"timestamp":"2024-04-05T13:30:00Z","is_open":true,"next_open":"2024-04-08T13:30:00Z","next_close":"2024-04-05T20:00:00Z"}"#
}

#[tokio::test]
async fn clock_get_hits_official_path_and_sends_auth_headers() {
    let server = TestServer::spawn(vec![format!(
        "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
        clock_json().len(),
        clock_json()
    )]);

    let clock = Client::builder()
        .api_key("key")
        .secret_key("secret")
        .base_url(server.base_url())
        .build()
        .expect("client should build")
        .clock()
        .get()
        .await
        .expect("clock request should succeed");

    assert_eq!(clock.timestamp, "2024-04-05T13:30:00Z");
    assert!(clock.is_open);

    let request = server.into_single_request();
    assert_eq!(request.request_line, "GET /v2/clock HTTP/1.1");
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
