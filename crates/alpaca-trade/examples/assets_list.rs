use alpaca_trade::Client;
use alpaca_trade::assets::ListRequest;

fn authenticated_client() -> Result<Client, Box<dyn std::error::Error>> {
    let api_key = std::env::var("APCA_API_KEY_ID")?;
    let secret_key = std::env::var("APCA_API_SECRET_KEY")?;

    let client = Client::builder()
        .api_key(api_key)
        .secret_key(secret_key)
        .paper()
        .build()?;

    Ok(client)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = authenticated_client()?;
    let assets = client
        .assets()
        .list(ListRequest {
            status: Some("active".to_owned()),
            asset_class: Some("us_equity".to_owned()),
            exchange: Some("NASDAQ".to_owned()),
            attributes: Some(vec!["has_options".to_owned()]),
        })
        .await?;

    for asset in assets.into_iter().take(5) {
        println!("{} {}", asset.symbol, asset.status);
    }

    Ok(())
}
