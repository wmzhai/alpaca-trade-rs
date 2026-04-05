mod support;

use alpaca_trade::{Client, calendar::ListRequest};

#[tokio::test]
async fn calendar_live_reads_paper_market_calendar() {
    let Some(credentials) = support::trade_credentials() else {
        eprintln!(
            "skipping live calendar test: missing ALPACA_TRADE_API_KEY / ALPACA_TRADE_SECRET_KEY or APCA_API_KEY_ID / APCA_API_SECRET_KEY"
        );
        return;
    };

    let calendar = Client::builder()
        .api_key(credentials.api_key)
        .secret_key(credentials.secret_key)
        .paper()
        .build()
        .expect("client should build")
        .calendar()
        .list(ListRequest {
            start: Some("2026-04-01".into()),
            end: Some("2026-04-03".into()),
        })
        .await
        .expect("live calendar request should succeed");

    assert!(!calendar.is_empty());

    let first = &calendar[0];
    assert!(!first.date.is_empty());
    assert!(!first.open.is_empty());
    assert!(!first.close.is_empty());
}
