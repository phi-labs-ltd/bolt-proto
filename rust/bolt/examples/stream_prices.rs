//! Usage: cargo run --example stream_prices --features stream-manager -- http://localhost:50051

use std::time::Duration;

use bolt_proto::bolt::prices::v2::{
    prices_service_client::PricesServiceClient, StreamPricesRequest,
};
use bolt_proto::stream_cache::{manager::spawn_price_manager, Config};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let endpoint = std::env::args()
        .nth(1)
        .unwrap_or("http://localhost:50051".into());
    let mut client = PricesServiceClient::connect(endpoint).await?;
    let stream = client
        .stream_prices(StreamPricesRequest { pairs: vec![] })
        .await?
        .into_inner();

    // safe after 2s, finalized after 10s, dropped 60s after receipt (or removal).
    let config = Config::new(
        Duration::from_secs(2),
        Duration::from_secs(10),
        Duration::from_secs(60),
        1024,
    )?;
    let manager = spawn_price_manager(stream, config);

    let mut events = manager.subscribe();
    tokio::spawn(async move {
        while let Ok(event) = events.recv().await {
            println!("{event:?}");
        }
    });

    // Returns when the stream ends or errors; reconnect by spawning a new manager.
    manager.closed().await?;
    Ok(())
}
