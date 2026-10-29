use std::time::Instant;

use super::{Cache, Config, Entry, Event, Status, StreamCache};
use crate::bolt::assets::v1::Pair;
use crate::bolt::prices::v2::{stream_prices_response::Update, Price, StreamPricesResponse};

/// Cache for `bolt.prices.v2.StreamPrices`, keyed by `Price.id`.
#[derive(Clone, Debug)]
pub struct PriceCache(Cache<Price>);

impl PriceCache {
    /// Creates an empty cache.
    pub fn new(config: Config) -> Self {
        Self(Cache::new(config))
    }

    /// The most recently received non-removed price of a pair.
    pub fn latest_for_pair(&self, pair: &Pair) -> Option<&Entry<Price>> {
        self.0
            .iter()
            .map(|(_, e)| e)
            .filter(|e| e.status != Status::Removed && e.value.pair.as_ref() == Some(pair))
            .max_by_key(|e| e.received_at)
    }
}

impl StreamCache for PriceCache {
    type Response = StreamPricesResponse;
    type Value = Price;

    fn apply(&mut self, response: StreamPricesResponse, now: Instant) -> Vec<Event> {
        match response.update {
            Some(Update::Price(p)) => self.0.apply_update(p.id.clone(), p, now),
            Some(Update::Removed(id)) => self.0.apply_removed(&id, now),
            None => None,
        }
        .into_iter()
        .collect()
    }

    fn cache(&self) -> &Cache<Price> {
        &self.0
    }

    fn cache_mut(&mut self) -> &mut Cache<Price> {
        &mut self.0
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    fn price(id: &str, base: &str) -> StreamPricesResponse {
        StreamPricesResponse {
            update: Some(Update::Price(Price {
                id: id.into(),
                pair: Some(Pair {
                    base: base.into(),
                    quote: "USDC".into(),
                }),
                ..Default::default()
            })),
        }
    }

    #[test]
    fn price_then_removed() {
        let s = Duration::from_secs(1);
        let mut c = PriceCache::new(Config::new(s, 2 * s, 3 * s).unwrap());
        let t0 = Instant::now();
        c.apply(price("p1", "SUI"), t0);
        c.apply(price("p2", "SUI"), t0 + s / 2);
        let pair = Pair {
            base: "SUI".into(),
            quote: "USDC".into(),
        };
        assert_eq!(c.latest_for_pair(&pair).unwrap().value.id, "p2");

        c.apply(
            StreamPricesResponse {
                update: Some(Update::Removed("p2".into())),
            },
            t0 + s,
        );
        assert_eq!(c.cache().get("p2").unwrap().status, Status::Removed);
        assert_eq!(c.latest_for_pair(&pair).unwrap().value.id, "p1");
    }
}
