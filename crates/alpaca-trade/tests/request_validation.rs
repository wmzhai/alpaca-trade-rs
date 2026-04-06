#[allow(dead_code)]
#[path = "../src/error.rs"]
mod internal_error;
#[path = "../src/common/validate.rs"]
mod internal_validate;

mod error {
    pub use super::internal_error::*;
}

use alpaca_trade::{Client, Error};

fn auth_client() -> Client {
    Client::builder()
        .api_key("test-key")
        .secret_key("test-secret")
        .base_url("http://127.0.0.1:9")
        .build()
        .expect("test client should build")
}

fn assert_public_invalid_request(error: Error, needles: &[&str]) {
    match error {
        Error::InvalidRequest(message) => {
            for needle in needles {
                assert!(
                    message.contains(needle),
                    "expected invalid request message {message:?} to contain {needle:?}"
                );
            }
        }
        other => panic!("expected invalid request error, got {other:?}"),
    }
}

fn assert_internal_invalid_request(error: internal_error::Error, needles: &[&str]) {
    match error {
        internal_error::Error::InvalidRequest(message) => {
            for needle in needles {
                assert!(
                    message.contains(needle),
                    "expected invalid request message {message:?} to contain {needle:?}"
                );
            }
        }
        other => panic!("expected invalid request error, got {other:?}"),
    }
}

#[test]
fn shared_limit_validation_rejects_zero() {
    let error = internal_validate::validate_limit(0, 500)
        .expect_err("zero limit should fail validation");

    assert_internal_invalid_request(error, &["limit", "greater than 0"]);
}

#[test]
fn shared_required_text_validation_rejects_blank_underlying_symbol() {
    let error = internal_validate::required_text("underlying_symbol", "   ")
        .expect_err("blank underlying_symbol should fail validation");

    assert_internal_invalid_request(error, &["underlying_symbol", "must not be blank"]);
}

#[tokio::test]
async fn assets_get_rejects_blank_symbol_or_asset_id_before_transport() {
    let error = auth_client()
        .assets()
        .get("   ")
        .await
        .expect_err("blank symbol_or_asset_id should fail before transport");

    assert_public_invalid_request(error, &["symbol_or_asset_id", "must not be blank"]);
}

#[tokio::test]
async fn assets_get_rejects_reserved_path_characters_before_transport() {
    for value in ["AAPL/US", "AAPL?draft=true", "AAPL#fragment", "AAPL%2FUS"] {
        let error = auth_client()
            .assets()
            .get(value)
            .await
            .expect_err("reserved path characters should fail before transport");

        assert_public_invalid_request(
            error,
            &["symbol_or_asset_id", "reserved path characters"],
        );
    }
}
