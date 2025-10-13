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
use std::collections::HashMap;
use std::env;
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

use once_cell::sync::OnceLock;
use opentelemetry::metrics::{CallbackRegistration, Meter, ObservableGauge, ObserverResult};
use opentelemetry::{global, KeyValue};

pub struct StalenessCfg {
    pub enable_recency: bool,
}

#[derive(Clone, Hash, Eq, PartialEq, Debug)]
pub struct SourceKey {
    pub source: String,
    pub domain: String,
}

#[derive(Clone)]
struct SourceState {
    last_arrival_sec: u64,
    last_origin_sec: Option<u64>,
}

struct StalenessInner {
    states: RwLock<HashMap<SourceKey, SourceState>>,
    service: String,
    env: String,
    enable_recency: bool,
}

impl StalenessInner {
    fn new(enable_recency: bool) -> Self {
        Self {
            states: RwLock::new(HashMap::new()),
            service: canonical_service_label(),
            env: canonical_env_label(),
            enable_recency,
        }
    }

    fn snapshot(&self, now: u64) -> Vec<(SourceKey, f64, Option<f64>)> {
        let guard = match self.states.read() {
            Ok(lock) => lock,
            Err(poisoned) => poisoned.into_inner(),
        };
        guard
            .iter()
            .map(|(key, state)| {
                let freshness = compute_age(now, state.last_arrival_sec);
                let recency = if self.enable_recency {
                    state
                        .last_origin_sec
                        .map(|origin| compute_age(now, origin))
                } else {
                    None
                };
                (key.clone(), freshness, recency)
            })
            .collect()
    }

    fn with_states_write<F>(&self, mut f: F)
    where
        F: FnMut(&mut HashMap<SourceKey, SourceState>),
    {
        match self.states.write() {
            Ok(mut guard) => f(&mut guard),
            Err(poisoned) => {
                let mut guard = poisoned.into_inner();
                f(&mut guard);
            }
        }
    }
}

pub struct StalenessRegistry {
    inner: Arc<StalenessInner>,
    freshness_gauge: ObservableGauge<f64>,
    recency_gauge: Option<ObservableGauge<f64>>,
    _callback: Option<CallbackRegistration>,
}

impl StalenessRegistry {
    pub fn new(meter: &Meter, cfg: StalenessCfg) -> Self {
        let inner = Arc::new(StalenessInner::new(cfg.enable_recency));
        let freshness_gauge = meter
            .f64_observable_gauge("data_freshness_seconds")
            .with_unit("s")
            .with_description("Age since last event arrival per source")
            .init();
        let recency_gauge = if cfg.enable_recency {
            Some(
                meter
                    .f64_observable_gauge("data_recency_seconds")
                    .with_unit("s")
                    .with_description("Age since last event origin per source")
                    .init(),
            )
        } else {
            None
        };
        let callback_inner = Arc::clone(&inner);
        let freshness_clone = freshness_gauge.clone();
        let recency_clone = recency_gauge.clone();
        let service_label = inner.service.clone();
        let env_label = inner.env.clone();
        let callback = meter
            .register_callback(move |observer: &mut ObserverResult<f64>| {
                let now = now_sec();
                let snapshot = callback_inner.snapshot(now);
                for (key, freshness, recency) in snapshot {
                    let attributes = [
                        KeyValue::new("source", key.source.clone()),
                        KeyValue::new("domain", key.domain.clone()),
                        KeyValue::new("service", service_label.as_str()),
                        KeyValue::new("env", env_label.as_str()),
                    ];
                    observer.observe(&freshness_clone, freshness, &attributes);
                    if let (Some(value), Some(gauge)) = (recency, recency_clone.as_ref()) {
                        observer.observe(gauge, value, &attributes);
                    }
                }
            })
            .ok();
        Self {
            inner,
            freshness_gauge,
            recency_gauge,
            _callback: callback,
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
        let now = now_sec();
        let key = SourceKey {
            source: source.to_string(),
            domain: domain.to_string(),
        };
        self.inner.with_states_write(|states| {
            let entry = states.entry(key).or_insert_with(|| SourceState {
                last_arrival_sec: now,
                last_origin_sec: None,
            });
            entry.last_arrival_sec = now;
        });
    }

    pub fn update_origin(&self, source: &str, domain: &str, origin_sec: u64) {
        let now = now_sec();
        let key = SourceKey {
            source: source.to_string(),
            domain: domain.to_string(),
        };
        self.inner.with_states_write(|states| {
            let entry = states.entry(key).or_insert_with(|| SourceState {
                last_arrival_sec: now,
                last_origin_sec: None,
            });
            entry.last_origin_sec = Some(origin_sec);
        });
    }

    pub fn metrics_snapshot(&self, now: u64) -> Vec<(SourceKey, f64, Option<f64>)> {
        self.inner.snapshot(now)
    }
}

fn canonical_service_label() -> String {
    match env::var("SERVICE_NAME") {
        Ok(value) => value.trim().to_string(),
        Err(_) => "ce-amm".to_string(),
    }
}

fn canonical_env_label() -> String {
    match env::var("DEPLOY_ENV") {
        Ok(raw) => {
            let lowered = raw.trim().to_ascii_lowercase();
            if lowered.is_empty() {
                "dev".to_string()
            } else {
                lowered
            }
        }
        Err(_) => "dev".to_string(),
    }
}

fn compute_age(now: u64, timestamp: u64) -> f64 {
    if now >= timestamp {
        (now - timestamp) as f64
    } else {
        0.0
    }
}

fn now_sec() -> u64 {
    const OVERRIDE_ENV: &str = "STALENESS_NOW_OVERRIDE";
    if let Ok(value) = env::var(OVERRIDE_ENV) {
        if let Ok(parsed) = value.trim().parse::<u64>() {
            return parsed;
        }
    }
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => duration.as_secs(),
        Err(_) => 0,
    }
}

static GLOBAL_REGISTRY: OnceLock<StalenessRegistry> = OnceLock::new();

pub fn global_staleness_registry() -> &'static StalenessRegistry {
    GLOBAL_REGISTRY.get_or_init(|| {
        let meter = global::meter("ce-obs");
        StalenessRegistry::new(&meter, StalenessCfg {
            enable_recency: false,
        })
    })
}
