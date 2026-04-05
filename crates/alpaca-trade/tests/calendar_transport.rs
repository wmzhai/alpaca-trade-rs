use alpaca_trade::Client;
use alpaca_trade::calendar::ListRequest;
mod support;

use support::TestServer;

fn calendar_json() -> &'static str {
    r#"[{"close":"16:00","date":"2026-04-01","open":"09:30","session_close":"2000","session_open":"0400","settlement_date":"2026-04-02"}]"#
}

#[tokio::test]
async fn calendar_list_hits_official_path_and_sends_auth_headers() {
    let server = TestServer::spawn(vec![format!(
        "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
        calendar_json().len(),
        calendar_json()
    )]);

    let calendar = Client::builder()
        .api_key("key")
        .secret_key("secret")
        .base_url(server.base_url())
        .build()
        .expect("client should build")
        .calendar()
        .list(ListRequest {
            start: Some("2026-04-01".to_owned()),
            end: Some("2026-04-03".to_owned()),
        })
        .await
        .expect("calendar request should succeed");

    assert_eq!(calendar.len(), 1);
    assert_eq!(calendar[0].date, "2026-04-01");

    let request = server.into_single_request();
    assert_eq!(
        request.request_line,
        "GET /v2/calendar?start=2026-04-01&end=2026-04-03 HTTP/1.1"
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
