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
