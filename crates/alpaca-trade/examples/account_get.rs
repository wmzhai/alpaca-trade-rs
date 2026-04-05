use alpaca_trade::Client;

#[tokio::main]
async fn main() -> Result<(), alpaca_trade::Error> {
    let client = Client::builder()
        .api_key(std::env::var("APCA_API_KEY_ID").expect("APCA_API_KEY_ID is required"))
        .secret_key(std::env::var("APCA_API_SECRET_KEY").expect("APCA_API_SECRET_KEY is required"))
        .build()?;

    let account = client.account().get().await?;
    println!("{}", account.status);
    Ok(())
}
