use alpaca_trade::Client;

#[tokio::main]
async fn main() -> Result<(), alpaca_trade::Error> {
    let client = Client::builder()
        .api_key(
            std::env::var("ALPACA_TRADE_API_KEY").expect("ALPACA_TRADE_API_KEY is required"),
        )
        .secret_key(
            std::env::var("ALPACA_TRADE_SECRET_KEY")
                .expect("ALPACA_TRADE_SECRET_KEY is required"),
        )
        .build()?;

    let account = client.account().get().await?;
    println!("{}", account.status);
    Ok(())
}
