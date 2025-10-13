use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug, PartialEq)]
pub struct StalenessSample {
    pub source: String,
    pub domain: String,
    pub freshness_seconds: f64,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct Key {
    source: String,
    domain: String,
}

#[derive(Clone, Debug)]
struct Entry {
    last_arrival: SystemTime,
    last_origin: Option<SystemTime>,
}

impl Entry {
    fn new(arrival: SystemTime) -> Self {
        Self {
            last_arrival: arrival,
            last_origin: None,
        }
    }

    fn reference_time(&self) -> SystemTime {
        self.last_origin.unwrap_or(self.last_arrival)
    }
}

#[derive(Clone, Debug)]
pub struct StalenessRegistry {
    entries: Arc<RwLock<HashMap<Key, Entry>>>,
}

impl StalenessRegistry {
    fn new() -> Self {
        Self {
            entries: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn update_arrival(&self, source: &str, domain: &str) {
        let arrival = SystemTime::now();
        let mut guard = self.entries.write().expect("staleness registry poisoned");
        let key = Key {
            source: source.to_string(),
            domain: domain.to_string(),
        };
        guard
            .entry(key)
            .and_modify(|entry| {
                entry.last_arrival = arrival;
                if let Some(origin) = entry.last_origin {
                    if origin > arrival {
                        entry.last_origin = Some(arrival);
                    }
                }
            })
            .or_insert_with(|| Entry::new(arrival));
    }

    pub fn update_origin(&self, source: &str, domain: &str, origin_sec: f64) {
        if !origin_sec.is_finite() {
            return;
        }
        if origin_sec < 0.0 {
            return;
        }
        let duration = Duration::from_secs_f64(origin_sec);
        let origin = match UNIX_EPOCH.checked_add(duration) {
            Some(ts) => ts,
            None => return,
        };
        let mut guard = self.entries.write().expect("staleness registry poisoned");
        let key = Key {
            source: source.to_string(),
            domain: domain.to_string(),
        };
        guard
            .entry(key)
            .and_modify(|entry| {
                entry.last_origin = Some(origin.min(entry.last_arrival));
            })
            .or_insert_with(|| Entry {
                last_arrival: origin,
                last_origin: Some(origin),
            });
    }

    pub fn snapshot(&self) -> Vec<StalenessSample> {
        let now = SystemTime::now();
        let guard = self.entries.read().expect("staleness registry poisoned");
        guard
            .iter()
            .map(|(key, entry)| {
                let reference = entry.reference_time();
                let freshness = match now.duration_since(reference) {
                    Ok(delta) => delta.as_secs_f64(),
                    Err(_) => 0.0,
                };
                StalenessSample {
                    source: key.source.clone(),
                    domain: key.domain.clone(),
                    freshness_seconds: freshness.max(0.0),
                }
            })
            .collect()
    }

    pub fn for_each<F: FnMut(&str, &str, f64)>(&self, mut callback: F) {
        for sample in self.snapshot() {
            callback(&sample.source, &sample.domain, sample.freshness_seconds);
        }
    }

    pub fn clear(&self) {
        let mut guard = self.entries.write().expect("staleness registry poisoned");
        guard.clear();
    }
}

static GLOBAL_REGISTRY: Lazy<StalenessRegistry> = Lazy::new(StalenessRegistry::new);

pub fn global_staleness_registry() -> &'static StalenessRegistry {
    &GLOBAL_REGISTRY
}
