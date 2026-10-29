use std::time::Instant;

use super::{Cache, Config, Event, StreamCache};
use crate::bolt::outpost::settlement::public::v3::{
    liquidity_update::Update, LiquidityUpdate, SubscribeLiquidityUpdatesResponse,
};

/// Cache for `bolt.outpost.settlement.public.v3.SubscribeLiquidityUpdates`, keyed by the
/// `tx_id` of each deposit, withdrawal or trade.
#[derive(Clone, Debug)]
pub struct LiquidityCache(Cache<LiquidityUpdate>);

impl LiquidityCache {
    /// Creates an empty cache.
    pub fn new(config: Config) -> Self {
        Self(Cache::new(config))
    }
}

impl StreamCache for LiquidityCache {
    type Response = SubscribeLiquidityUpdatesResponse;
    type Value = LiquidityUpdate;

    fn apply(&mut self, response: SubscribeLiquidityUpdatesResponse, now: Instant) -> Vec<Event> {
        response
            .updates
            .into_iter()
            .filter_map(|u| {
                let id = match u.update.as_ref()? {
                    Update::Deposit(d) => d.tx_id.clone(),
                    Update::Withdrawal(w) => w.tx_id.clone(),
                    Update::Trade(t) => t.tx_id.clone(),
                    Update::Removed(id) => return self.0.apply_removed(id, now),
                };
                self.0.apply_update(id, u, now)
            })
            .collect()
    }

    fn cache(&self) -> &Cache<LiquidityUpdate> {
        &self.0
    }

    fn cache_mut(&mut self) -> &mut Cache<LiquidityUpdate> {
        &mut self.0
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::bolt::outpost::settlement::public::v3::{Deposit, Trade, Withdrawal};
    use crate::stream_cache::Status;

    fn lu(u: Update) -> LiquidityUpdate {
        LiquidityUpdate { update: Some(u) }
    }

    #[test]
    fn all_kinds_then_removed() {
        let s = Duration::from_secs(1);
        let mut c = LiquidityCache::new(Config::new(s, 2 * s, 3 * s).unwrap());
        let t0 = Instant::now();
        let ev = c.apply(
            SubscribeLiquidityUpdatesResponse {
                block_height: 7,
                updates: vec![
                    lu(Update::Deposit(Deposit {
                        tx_id: "d".into(),
                        ..Default::default()
                    })),
                    lu(Update::Withdrawal(Withdrawal {
                        tx_id: "w".into(),
                        ..Default::default()
                    })),
                    lu(Update::Trade(Trade {
                        tx_id: "t".into(),
                        ..Default::default()
                    })),
                ],
            },
            t0,
        );
        assert_eq!(ev.len(), 3);
        assert_eq!(c.cache().len(), 3);

        let ev = c.apply(
            SubscribeLiquidityUpdatesResponse {
                block_height: 8,
                updates: vec![lu(Update::Removed("t".into()))],
            },
            t0 + s,
        );
        assert_eq!(ev.len(), 1);
        assert_eq!(c.cache().get("t").unwrap().status, Status::Removed);
        assert_eq!(c.cache().get("d").unwrap().status, Status::Latest);
    }
}
