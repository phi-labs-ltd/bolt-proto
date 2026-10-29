use std::collections::HashMap;
use std::time::{Duration, Instant};

mod liquidity;
mod prices;

#[cfg(feature = "stream-manager")]
pub mod manager;

pub use liquidity::LiquidityCache;
pub use prices::PriceCache;

/// Lifecycle status of a cached update.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Status {
    /// Just received; may still be reorged.
    Latest,
    /// Older than `safe_period`; a reorg is unlikely.
    Safe,
    /// Older than `finalization_period`; considered final.
    Finalized,
    /// The stream reported this update as reverted.
    Removed,
}

/// Invalid [`Config`] periods.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConfigError {
    /// `safe_period` must be strictly less than `finalization_period`.
    SafeNotBeforeFinalization,
    /// `prune_after` must be strictly greater than `finalization_period`.
    PruneNotAfterFinalization,
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SafeNotBeforeFinalization => {
                f.write_str("safe_period must be less than finalization_period")
            }
            Self::PruneNotAfterFinalization => {
                f.write_str("prune_after must be greater than finalization_period")
            }
        }
    }
}

impl std::error::Error for ConfigError {}

/// Timing configuration: `safe_period < finalization_period < prune_after`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Config {
    safe_period: Duration,
    finalization_period: Duration,
    prune_after: Duration,
}

impl Config {
    /// Validates and builds a config.
    pub fn new(
        safe_period: Duration,
        finalization_period: Duration,
        prune_after: Duration,
    ) -> Result<Self, ConfigError> {
        if safe_period >= finalization_period {
            return Err(ConfigError::SafeNotBeforeFinalization);
        }
        if prune_after <= finalization_period {
            return Err(ConfigError::PruneNotAfterFinalization);
        }
        Ok(Self {
            safe_period,
            finalization_period,
            prune_after,
        })
    }

    /// Age after which an entry becomes [`Status::Safe`].
    pub fn safe_period(&self) -> Duration {
        self.safe_period
    }

    /// Age after which an entry becomes [`Status::Finalized`].
    pub fn finalization_period(&self) -> Duration {
        self.finalization_period
    }

    /// Age after which an entry is dropped (counted from removal for removed entries).
    pub fn prune_after(&self) -> Duration {
        self.prune_after
    }
}

/// A cached update with its status.
#[derive(Clone, Debug, PartialEq)]
pub struct Entry<T> {
    /// The cached update.
    pub value: T,
    /// Current status.
    pub status: Status,
    /// When the update was first received.
    pub received_at: Instant,
    /// When the update was reported removed, if it was.
    pub removed_at: Option<Instant>,
}

/// A change to the cache, reported to observers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    /// A new or refreshed update with this id was stored.
    Updated { id: String },
    /// The status of the entry changed (including to [`Status::Removed`]).
    StatusChanged {
        id: String,
        from: Status,
        to: Status,
    },
    /// The entry was dropped from the cache.
    Pruned { id: String },
}

/// Cache of updates keyed by id.
#[derive(Clone, Debug)]
pub struct Cache<T> {
    config: Config,
    entries: HashMap<String, Entry<T>>,
}

impl<T> Cache<T> {
    /// Creates an empty cache.
    pub fn new(config: Config) -> Self {
        Self {
            config,
            entries: HashMap::new(),
        }
    }

    /// The cache's timing configuration.
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Stores an update as [`Status::Latest`]. Re-sending a known id replaces the value but keeps
    /// its status and timers; an id already marked [`Status::Removed`] is ignored.
    pub fn apply_update(&mut self, id: String, value: T, now: Instant) -> Option<Event> {
        match self.entries.get_mut(&id) {
            Some(entry) if entry.status == Status::Removed => None,
            Some(entry) => {
                entry.value = value;
                Some(Event::Updated { id })
            }
            None => {
                self.entries.insert(
                    id.clone(),
                    Entry {
                        value,
                        status: Status::Latest,
                        received_at: now,
                        removed_at: None,
                    },
                );
                Some(Event::Updated { id })
            }
        }
    }

    /// Marks the update with this id as [`Status::Removed`]. Unknown or already removed ids are
    /// ignored.
    pub fn apply_removed(&mut self, id: &str, now: Instant) -> Option<Event> {
        let entry = self.entries.get_mut(id)?;
        if entry.status == Status::Removed {
            return None;
        }
        let from = std::mem::replace(&mut entry.status, Status::Removed);
        entry.removed_at = Some(now);
        Some(Event::StatusChanged {
            id: id.to_owned(),
            from,
            to: Status::Removed,
        })
    }

    /// Advances statuses and prunes expired entries as of `now`.
    pub fn tick(&mut self, now: Instant) -> Vec<Event> {
        let config = self.config;
        let mut events = Vec::new();
        self.entries.retain(|id, entry| {
            let expiry = entry.removed_at.unwrap_or(entry.received_at) + config.prune_after;
            if now >= expiry {
                events.push(Event::Pruned { id: id.clone() });
                return false;
            }
            if entry.status != Status::Removed {
                let age = now.saturating_duration_since(entry.received_at);
                let to = if age >= config.finalization_period {
                    Status::Finalized
                } else if age >= config.safe_period {
                    Status::Safe
                } else {
                    Status::Latest
                };
                if to != entry.status {
                    events.push(Event::StatusChanged {
                        id: id.clone(),
                        from: entry.status,
                        to,
                    });
                    entry.status = to;
                }
            }
            true
        });
        events
    }

    /// The next instant at which [`tick`](Self::tick) would change something, if any.
    pub fn next_deadline(&self) -> Option<Instant> {
        self.entries
            .values()
            .map(|e| match e.status {
                Status::Latest => e.received_at + self.config.safe_period,
                Status::Safe => e.received_at + self.config.finalization_period,
                Status::Finalized => e.received_at + self.config.prune_after,
                Status::Removed => e.removed_at.unwrap_or(e.received_at) + self.config.prune_after,
            })
            .min()
    }

    /// The entry with this id.
    pub fn get(&self, id: &str) -> Option<&Entry<T>> {
        self.entries.get(id)
    }

    /// All entries, in arbitrary order.
    pub fn iter(&self) -> impl Iterator<Item = (&String, &Entry<T>)> {
        self.entries.iter()
    }

    /// Number of cached entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// A [`Cache`] fed by one gRPC stream's response type.
pub trait StreamCache {
    /// The stream's response message.
    type Response;
    /// The cached value.
    type Value: Clone;

    /// Applies one streamed response.
    fn apply(&mut self, response: Self::Response, now: Instant) -> Vec<Event>;
    /// The underlying cache.
    fn cache(&self) -> &Cache<Self::Value>;
    /// The underlying cache, mutably.
    fn cache_mut(&mut self) -> &mut Cache<Self::Value>;
}

#[cfg(test)]
mod tests {
    use super::*;

    const S: Duration = Duration::from_secs(1);

    fn cfg() -> Config {
        Config::new(10 * S, 30 * S, 60 * S).unwrap()
    }

    #[test]
    fn config_validation() {
        assert_eq!(
            Config::new(30 * S, 30 * S, 60 * S),
            Err(ConfigError::SafeNotBeforeFinalization)
        );
        assert_eq!(
            Config::new(10 * S, 30 * S, 30 * S),
            Err(ConfigError::PruneNotAfterFinalization)
        );
    }

    #[test]
    fn ages_through_statuses_then_prunes() {
        let t0 = Instant::now();
        let mut c = Cache::new(cfg());
        c.apply_update("a".into(), 1, t0);
        assert_eq!(c.get("a").unwrap().status, Status::Latest);
        assert_eq!(c.next_deadline(), Some(t0 + 10 * S));

        assert!(c.tick(t0 + 9 * S).is_empty());
        c.tick(t0 + 10 * S);
        assert_eq!(c.get("a").unwrap().status, Status::Safe);
        c.tick(t0 + 30 * S);
        assert_eq!(c.get("a").unwrap().status, Status::Finalized);
        assert_eq!(c.next_deadline(), Some(t0 + 60 * S));
        assert_eq!(c.tick(t0 + 60 * S), vec![Event::Pruned { id: "a".into() }]);
        assert!(c.is_empty());
    }

    #[test]
    fn late_tick_jumps_straight_to_finalized() {
        let t0 = Instant::now();
        let mut c = Cache::new(cfg());
        c.apply_update("a".into(), 1, t0);
        let ev = c.tick(t0 + 40 * S);
        assert_eq!(
            ev,
            vec![Event::StatusChanged {
                id: "a".into(),
                from: Status::Latest,
                to: Status::Finalized
            }]
        );
    }

    #[test]
    fn removed_prunes_after_removal_not_receipt() {
        let t0 = Instant::now();
        let mut c = Cache::new(cfg());
        c.apply_update("a".into(), 1, t0);
        c.apply_removed("a", t0 + 5 * S).unwrap();
        assert_eq!(c.get("a").unwrap().status, Status::Removed);
        // Would have been pruned at 60s from receipt, but removal restarts the clock.
        assert!(c.tick(t0 + 64 * S).is_empty());
        assert_eq!(c.get("a").unwrap().status, Status::Removed);
        assert_eq!(c.tick(t0 + 65 * S).len(), 1);
        assert!(c.is_empty());
    }

    #[test]
    fn removed_unknown_or_twice_is_noop() {
        let t0 = Instant::now();
        let mut c: Cache<u8> = Cache::new(cfg());
        assert!(c.apply_removed("x", t0).is_none());
        c.apply_update("a".into(), 1, t0);
        assert!(c.apply_removed("a", t0).is_some());
        assert!(c.apply_removed("a", t0 + S).is_none());
        assert_eq!(c.get("a").unwrap().removed_at, Some(t0));
    }

    #[test]
    fn resend_keeps_timers_and_removed_is_not_resurrected() {
        let t0 = Instant::now();
        let mut c = Cache::new(cfg());
        c.apply_update("a".into(), 1, t0);
        c.tick(t0 + 10 * S);
        c.apply_update("a".into(), 2, t0 + 11 * S);
        let e = c.get("a").unwrap();
        assert_eq!((e.value, e.status, e.received_at), (2, Status::Safe, t0));

        c.apply_removed("a", t0 + 12 * S);
        assert!(c.apply_update("a".into(), 3, t0 + 13 * S).is_none());
        assert_eq!(c.get("a").unwrap().value, 2);
    }
}
