use alpaca_trade::Client;

#[tokio::test]
async fn client_reads_account_from_mock_server() {
    let server = alpaca_trade_mock::spawn_test_server().await;

    let account = Client::builder()
        .api_key("key")
        .secret_key("secret")
        .base_url(server.base_url)
        .build()
        .expect("client should build")
        .account()
        .get()
        .await
        .expect("request should succeed");

    assert_eq!(account.status, "ACTIVE");
    assert_eq!(account.currency.as_deref(), Some("USD"));
}
