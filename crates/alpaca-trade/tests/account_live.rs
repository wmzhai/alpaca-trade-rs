mod support;

use alpaca_trade::Client;

#[tokio::test]
async fn account_live_reads_paper_account() {
    let env_guard = support::env_lock();
    let Some(credentials) = support::trade_credentials() else {
        drop(env_guard);
        eprintln!(
            "skipping live account test: missing ALPACA_TRADE_API_KEY or ALPACA_TRADE_SECRET_KEY"
        );
        return;
    };
    drop(env_guard);

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
