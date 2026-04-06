use alpaca_trade::Client;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = std::env::var("APCA_API_KEY_ID")?;
    let secret_key = std::env::var("APCA_API_SECRET_KEY")?;

    let _client = Client::builder()
        .api_key(api_key)
        .secret_key(secret_key)
        .paper()
        .build()?;

    println!("paper client configured");
    Ok(())
}
