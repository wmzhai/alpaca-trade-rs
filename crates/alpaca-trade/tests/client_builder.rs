use alpaca_trade::{Client, Error};

#[test]
fn builder_builds_paper_client_by_default() {
    let client = Client::builder()
        .api_key("key")
        .secret_key("secret")
        .build()
        .expect("paper client should build");

    let _ = client.account();
}

#[test]
fn builder_rejects_partial_credentials() {
    let error = Client::builder()
        .api_key("key-only")
        .build()
        .expect_err("partial credentials must fail");

    assert!(matches!(
        error,
        Error::InvalidConfiguration(message)
            if message.contains("api_key") && message.contains("secret_key")
    ));
}

#[test]
fn builder_rejects_missing_credentials() {
    let error = Client::builder()
        .build()
        .expect_err("missing credentials must fail");

    assert!(matches!(error, Error::MissingCredentials));
}

#[test]
fn builder_rejects_whitespace_only_credentials() {
    let error = Client::builder()
        .api_key("   ")
        .secret_key("\t")
        .build()
        .expect_err("blank credentials must fail");

    assert!(matches!(
        error,
        Error::InvalidConfiguration(message)
            if message.contains("api_key") || message.contains("secret_key")
    ));
}

#[test]
fn builder_rejects_invalid_base_url() {
    let error = Client::builder()
        .api_key("key")
        .secret_key("secret")
        .base_url("not a url")
        .build()
        .expect_err("invalid base_url must fail");

    assert!(matches!(
        error,
        Error::InvalidConfiguration(message) if message.contains("base_url")
    ));
}

#[test]
fn client_exposes_account_accessor() {
    let client = Client::builder()
        .api_key("key")
        .secret_key("secret")
        .build()
        .expect("client should build");

    let _ = client.account();
}
