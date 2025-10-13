use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StalenessSnapshot {
    pub staleness_seconds: f64,
    pub freshness_seconds: f64,
    pub recency_seconds: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct StalenessClock {
    last_arrival: Option<Instant>,
    staleness_seconds: f64,
}

impl StalenessClock {
    pub fn new() -> Self {
        Self {
            last_arrival: None,
            staleness_seconds: 0.0,
        }
    }

    pub fn with_initial_arrival(at: Instant) -> Self {
        let mut clock = Self::new();
        clock.last_arrival = Some(at);
        clock
    }

    pub fn on_arrival(&mut self, at: Instant) -> StalenessSnapshot {
        self.last_arrival = Some(at);
        self.staleness_seconds = 0.0;
        StalenessSnapshot {
            staleness_seconds: 0.0,
            freshness_seconds: 0.0,
            recency_seconds: Some(0.0),
        }
    }

    pub fn on_clock_step(&mut self, now: Instant) -> StalenessSnapshot {
        let recency_seconds = self.last_arrival.map(|arrival| {
            now.checked_duration_since(arrival)
                .unwrap_or_else(|| Duration::from_secs(0))
                .as_secs_f64()
        });
        let staleness_seconds = recency_seconds.unwrap_or(self.staleness_seconds);
        let staleness_seconds = staleness_seconds.max(0.0).max(self.staleness_seconds);
        self.staleness_seconds = staleness_seconds;
        StalenessSnapshot {
            staleness_seconds,
            freshness_seconds: staleness_seconds,
            recency_seconds,
        }
    }

    pub fn recency_seconds(&self, now: Instant) -> Option<f64> {
        self.last_arrival.map(|arrival| {
            now.checked_duration_since(arrival)
                .unwrap_or_else(|| Duration::from_secs(0))
                .as_secs_f64()
        })
    }

    pub fn staleness_seconds(&self) -> f64 {
        self.staleness_seconds
    }
}
