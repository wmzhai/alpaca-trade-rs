mod support;

use alpaca_trade::Client;

#[tokio::test]
async fn account_live_reads_paper_account() {
    let Some(credentials) = support::trade_credentials() else {
        eprintln!(
            "skipping live account test: missing ALPACA_TRADE_API_KEY / ALPACA_TRADE_SECRET_KEY or APCA_API_KEY_ID / APCA_API_SECRET_KEY"
        );
        return;
    };

    let account = Client::builder()
        .api_key(credentials.api_key)
        .secret_key(credentials.secret_key)
        .paper()
        .build()
        .expect("client should build")
        .account()
        .get()
        .await
        .expect("live account request should succeed");

    assert!(!account.id.is_empty());
    assert!(!account.account_number.is_empty());
    assert!(!account.status.is_empty());
}
