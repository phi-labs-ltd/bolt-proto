use std::sync::{Arc, RwLock};
use std::time::Instant;

use tokio::sync::{broadcast, watch};
use tokio_stream::{Stream, StreamExt};

use super::{Config, Entry, Event, LiquidityCache, PriceCache, StreamCache};
use crate::bolt::outpost::settlement::public::v3::SubscribeLiquidityUpdatesResponse;
use crate::bolt::prices::v2::StreamPricesResponse;

/// Manager for `StreamPrices`.
pub type PriceManager = Manager<PriceCache>;
/// Manager for `SubscribeLiquidityUpdates`.
pub type LiquidityManager = Manager<LiquidityCache>;

/// Spawns a [`PriceManager`] consuming `stream`. Must be called inside a tokio runtime.
pub fn spawn_price_manager<S>(stream: S, config: Config) -> PriceManager
where
    S: Stream<Item = Result<StreamPricesResponse, tonic::Status>> + Send + 'static,
{
    Manager::spawn(PriceCache::new(config), stream)
}

/// Spawns a [`LiquidityManager`] consuming `stream`. Must be called inside a tokio runtime.
pub fn spawn_liquidity_manager<S>(stream: S, config: Config) -> LiquidityManager
where
    S: Stream<Item = Result<SubscribeLiquidityUpdatesResponse, tonic::Status>> + Send + 'static,
{
    Manager::spawn(LiquidityCache::new(config), stream)
}

struct Inner<C> {
    state: Arc<RwLock<C>>,
    events: broadcast::Sender<Event>,
    done: watch::Receiver<Option<Result<(), tonic::Status>>>,
    shutdown: watch::Sender<bool>,
}

/// Cloneable handle to a running cache. The background task stops when [`shutdown`](Self::shutdown)
/// is called, every handle is dropped, or the stream ends.
pub struct Manager<C: StreamCache> {
    inner: Arc<Inner<C>>,
}

impl<C: StreamCache> Clone for Manager<C> {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl<C> Manager<C>
where
    C: StreamCache + Send + Sync + 'static,
    C::Response: Send + 'static,
{
    fn spawn<S>(cache: C, stream: S) -> Self
    where
        S: Stream<Item = Result<C::Response, tonic::Status>> + Send + 'static,
    {
        let state = Arc::new(RwLock::new(cache));
        let (events, _) = broadcast::channel(cache.cache().config().event_buffer());
        let (done_tx, done) = watch::channel(None);
        let (shutdown, shutdown_rx) = watch::channel(false);
        tokio::spawn(run(
            Arc::clone(&state),
            stream,
            events.clone(),
            done_tx,
            shutdown_rx,
        ));
        Self {
            inner: Arc::new(Inner {
                state,
                events,
                done,
                shutdown,
            }),
        }
    }
}

impl<C: StreamCache> Manager<C> {
    /// The entry with this id, if cached.
    pub fn get(&self, id: &str) -> Option<Entry<C::Value>> {
        self.read(|c| c.cache().get(id).cloned())
    }

    /// A copy of all entries.
    pub fn snapshot(&self) -> Vec<(String, Entry<C::Value>)> {
        self.read(|c| {
            c.cache()
                .iter()
                .map(|(id, e)| (id.clone(), e.clone()))
                .collect()
        })
    }

    /// Runs `f` on the cache under a read lock, for typed lookups such as
    /// [`PriceCache::latest_for_pair`]. Don't block inside `f`.
    pub fn read<R>(&self, f: impl FnOnce(&C) -> R) -> R {
        f(&self.inner.state.read().unwrap_or_else(|e| e.into_inner()))
    }

    /// Subscribes to cache changes. Slow receivers observe `RecvError::Lagged`.
    pub fn subscribe(&self) -> broadcast::Receiver<Event> {
        self.inner.events.subscribe()
    }

    /// Asks the background task to stop.
    pub fn shutdown(&self) {
        let _ = self.inner.shutdown.send(true);
    }

    /// Resolves when the task has stopped: `Ok(())` on clean end or shutdown, `Err` if the stream
    /// failed.
    pub async fn closed(&self) -> Result<(), tonic::Status> {
        let mut done = self.inner.done.clone();
        let result = match done.wait_for(Option::is_some).await {
            Ok(v) => v.clone().unwrap_or(Ok(())),
            Err(_) => Ok(()),
        };
        result
    }
}

fn now() -> Instant {
    // Follows tokio's (possibly paused) clock so timers and the cache agree.
    tokio::time::Instant::now().into_std()
}

async fn run<C, S>(
    state: Arc<RwLock<C>>,
    stream: S,
    events: broadcast::Sender<Event>,
    done: watch::Sender<Option<Result<(), tonic::Status>>>,
    mut shutdown: watch::Receiver<bool>,
) where
    C: StreamCache,
    S: Stream<Item = Result<C::Response, tonic::Status>>,
{
    tokio::pin!(stream);
    let result = loop {
        let deadline = state
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .cache()
            .next_deadline();
        let sleep = async {
            match deadline {
                Some(d) => tokio::time::sleep_until(tokio::time::Instant::from_std(d)).await,
                None => std::future::pending().await,
            }
        };
        let evs = tokio::select! {
            // Stops on shutdown(), or when the last handle (and so the sender) is dropped.
            _ = shutdown.changed() => break Ok(()),
            item = stream.next() => match item {
                Some(Ok(resp)) => state
                    .write()
                    .unwrap_or_else(|e| e.into_inner())
                    .apply(resp, now()),
                Some(Err(status)) => break Err(status),
                None => break Ok(()),
            },
            _ = sleep => state
                .write()
                .unwrap_or_else(|e| e.into_inner())
                .cache_mut()
                .tick(now()),
        };
        for ev in evs {
            let _ = events.send(ev);
        }
    };
    let _ = done.send(Some(result));
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use tokio::sync::mpsc;
    use tokio_stream::wrappers::ReceiverStream;

    use super::*;
    use crate::bolt::prices::v2::{stream_prices_response::Update, Price};
    use crate::stream_cache::Status;

    fn price(id: &str) -> Result<StreamPricesResponse, tonic::Status> {
        Ok(StreamPricesResponse {
            update: Some(Update::Price(Price {
                id: id.into(),
                ..Default::default()
            })),
        })
    }

    fn cfg() -> Config {
        let s = Duration::from_secs(1);
        Config::new(10 * s, 30 * s, 60 * s).unwrap()
    }

    #[tokio::test(start_paused = true)]
    async fn drives_statuses_and_events() {
        let (tx, rx) = mpsc::channel(8);
        let m = spawn_price_manager(ReceiverStream::new(rx), cfg());
        let mut events = m.subscribe();

        tx.send(price("p1")).await.unwrap();
        assert_eq!(
            events.recv().await.unwrap(),
            Event::Updated { id: "p1".into() }
        );
        assert_eq!(m.get("p1").unwrap().status, Status::Latest);

        tokio::time::sleep(Duration::from_secs(11)).await;
        assert_eq!(m.get("p1").unwrap().status, Status::Safe);
        tokio::time::sleep(Duration::from_secs(20)).await;
        assert_eq!(m.get("p1").unwrap().status, Status::Finalized);

        tx.send(price("p2")).await.unwrap();
        tx.send(Ok(StreamPricesResponse {
            update: Some(Update::Removed("p2".into())),
        }))
        .await
        .unwrap();
        tokio::time::sleep(Duration::from_secs(1)).await;
        assert_eq!(m.get("p2").unwrap().status, Status::Removed);

        tokio::time::sleep(Duration::from_secs(60)).await;
        assert!(m.get("p1").is_none() && m.get("p2").is_none());
        assert!(m.snapshot().is_empty());
    }

    #[tokio::test(start_paused = true)]
    async fn reports_stream_end_and_error() {
        let (tx, rx) = mpsc::channel(8);
        let m = spawn_price_manager(ReceiverStream::new(rx), cfg());
        tx.send(Err(tonic::Status::unavailable("boom")))
            .await
            .unwrap();
        let err = m.closed().await.unwrap_err();
        assert_eq!(err.code(), tonic::Code::Unavailable);

        let (tx, rx) = mpsc::channel(8);
        let m = spawn_price_manager(ReceiverStream::new(rx), cfg());
        drop(tx);
        assert!(m.closed().await.is_ok());
    }

    #[tokio::test(start_paused = true)]
    async fn shutdown_stops_task() {
        let (_tx, rx) = mpsc::channel(8);
        let m = spawn_price_manager(ReceiverStream::new(rx), cfg());
        m.shutdown();
        assert!(m.closed().await.is_ok());
    }
}
