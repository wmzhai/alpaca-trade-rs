mod support;

use alpaca_trade::Client;

#[tokio::test]
async fn clock_live_reads_paper_market_clock() {
    let Some(credentials) = support::trade_credentials() else {
        eprintln!(
            "skipping live clock test: missing ALPACA_TRADE_API_KEY / ALPACA_TRADE_SECRET_KEY or APCA_API_KEY_ID / APCA_API_SECRET_KEY"
        );
        return;
    };

    let clock = Client::builder()
        .api_key(credentials.api_key)
        .secret_key(credentials.secret_key)
        .paper()
        .build()
        .expect("client should build")
        .clock()
        .get()
        .await
        .expect("live clock request should succeed");

    assert!(!clock.timestamp.is_empty());
    assert!(!clock.next_open.is_empty());
    assert!(!clock.next_close.is_empty());
    let _ = clock.is_open;
}
