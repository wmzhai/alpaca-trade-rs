use std::sync::OnceLock;

static DOTENV: OnceLock<()> = OnceLock::new();

#[derive(Debug, Clone)]
pub struct Credentials {
    pub api_key: String,
    pub secret_key: String,
}

pub fn trade_credentials() -> Option<Credentials> {
    DOTENV.get_or_init(|| {
        let _ = dotenvy::dotenv();
    });

    let api_key = std::env::var("ALPACA_TRADE_API_KEY").ok()?;
    let secret_key = std::env::var("ALPACA_TRADE_SECRET_KEY").ok()?;

    Some(Credentials {
        api_key,
        secret_key,
    })
}
