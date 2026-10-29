//! Usage: cargo run --example subscribe_liquidity_updates --features stream-manager -- http://localhost:50051

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use bolt_proto::bolt::outpost::settlement::public::v3::{
    public_settlement_service_client::PublicSettlementServiceClient,
    SubscribeLiquidityUpdatesRequest,
};
use bolt_proto::stream_cache::{manager::spawn_liquidity_manager, Config, Status};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let endpoint = std::env::args()
        .nth(1)
        .unwrap_or("http://localhost:50051".into());
    let mut client = PublicSettlementServiceClient::connect(endpoint).await?;
    let stream = client
        .subscribe_liquidity_updates(SubscribeLiquidityUpdatesRequest { field_mask: None })
        .await?
        .into_inner();

    let config = Config::new(
        Duration::from_secs(2),
        Duration::from_secs(10),
        Duration::from_secs(60),
        1024,
    )?;
    let manager = spawn_liquidity_manager(stream, config);

    // Entries are keyed by tx_id; only act on updates that can no longer be reorged.
    let mut events = manager.subscribe();
    let handle = manager.clone();
    tokio::spawn(async move {
        while let Ok(event) = events.recv().await {
            if let bolt_proto::stream_cache::Event::StatusChanged {
                id,
                to: Status::Finalized,
                ..
            } = event
            {
                println!("finalized: {id} {:?}", handle.get(&id).map(|e| e.value));
            }
        }
    });

    // Every few seconds, look up one cached update by id and print its current status.
    let sampler = manager.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(5)).await;
            let entries = sampler.snapshot();
            if entries.is_empty() {
                continue;
            }
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .subsec_nanos();
            let (id, _) = &entries[nanos as usize % entries.len()];
            if let Some(entry) = sampler.get(id) {
                println!(
                    "sampled {id}: {:?} (cached: {})",
                    entry.status,
                    entries.len()
                );
            }
        }
    });

    manager.closed().await?;
    Ok(())
}
