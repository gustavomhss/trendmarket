use crate::obs::staleness::global_staleness_registry;
use std::error::Error;
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;

#[derive(Debug, Clone)]
pub struct OracleEvent {
    pub instrument: String,
    pub best_bid: f64,
    pub best_ask: f64,
    pub origin_timestamp: Option<SystemTime>,
}

#[derive(Debug, Clone)]
pub struct MarketFeedEvent {
    pub venue: String,
    pub symbol: String,
    pub last_price: f64,
    pub size: f64,
    pub sequence: u64,
    pub exchange_timestamp: Option<SystemTime>,
}

#[derive(Debug, Clone)]
pub struct CdcOrderEvent {
    pub order_id: String,
    pub change: OrderChange,
    pub commit_ts: SystemTime,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderChange {
    Created,
    Updated,
    Deleted,
}

#[derive(Debug, Clone)]
pub struct ChainHeader {
    pub height: u64,
    pub hash: [u8; 32],
    pub parent_hash: [u8; 32],
    pub timestamp: Option<SystemTime>,
}

#[derive(Debug, Error)]
pub enum HandlerError {
    #[error("invalid event: {0}")]
    Invalid(String),
    #[error("ack failed: {0}")]
    Ack(String),
}

fn epoch_seconds(time: SystemTime) -> Result<f64, HandlerError> {
    let duration = time
        .duration_since(UNIX_EPOCH)
        .map_err(|_| HandlerError::Invalid("timestamp before epoch".to_string()))?;
    Ok(duration.as_secs_f64())
}

pub fn handle_oracle_event<F, E>(event: OracleEvent, ack: F) -> Result<(), HandlerError>
where
    F: FnOnce(&OracleEvent) -> Result<(), E>,
    E: Error + Send + Sync + 'static,
{
    if event.instrument.trim().is_empty() {
        return Err(HandlerError::Invalid("instrument missing".to_string()));
    }
    if !event.best_bid.is_finite() || !event.best_ask.is_finite() {
        return Err(HandlerError::Invalid("price not finite".to_string()));
    }
    if event.best_bid <= 0.0 || event.best_ask <= 0.0 {
        return Err(HandlerError::Invalid("price must be positive".to_string()));
    }
    if event.best_bid > event.best_ask {
        return Err(HandlerError::Invalid("bid greater than ask".to_string()));
    }
    let registry = global_staleness_registry();
    registry.update_arrival("oracle", "pricing");
    if let Some(origin) = event.origin_timestamp {
        let origin_sec = epoch_seconds(origin)?;
        registry.update_origin("oracle", "pricing", origin_sec);
    }
    ack(&event).map_err(|err| HandlerError::Ack(err.to_string()))
}

pub fn handle_market_feed<F, E>(event: MarketFeedEvent, ack: F) -> Result<(), HandlerError>
where
    F: FnOnce(&MarketFeedEvent) -> Result<(), E>,
    E: Error + Send + Sync + 'static,
{
    if event.venue.trim().is_empty() {
        return Err(HandlerError::Invalid("venue missing".to_string()));
    }
    if event.symbol.trim().is_empty() {
        return Err(HandlerError::Invalid("symbol missing".to_string()));
    }
    if event.sequence == 0 {
        return Err(HandlerError::Invalid(
            "sequence must be positive".to_string(),
        ));
    }
    if !event.last_price.is_finite() || event.last_price <= 0.0 {
        return Err(HandlerError::Invalid("last price invalid".to_string()));
    }
    if !event.size.is_finite() || event.size < 0.0 {
        return Err(HandlerError::Invalid("size invalid".to_string()));
    }
    let registry = global_staleness_registry();
    registry.update_arrival("market_feed", "market");
    if let Some(origin) = event.exchange_timestamp {
        let origin_sec = epoch_seconds(origin)?;
        registry.update_origin("market_feed", "market", origin_sec);
    }
    ack(&event).map_err(|err| HandlerError::Ack(err.to_string()))
}

pub fn handle_cdc_orders<F, E>(event: CdcOrderEvent, ack: F) -> Result<(), HandlerError>
where
    F: FnOnce(&CdcOrderEvent) -> Result<(), E>,
    E: Error + Send + Sync + 'static,
{
    if event.order_id.trim().is_empty() {
        return Err(HandlerError::Invalid("order id missing".to_string()));
    }
    let commit_sec = epoch_seconds(event.commit_ts)?;
    let registry = global_staleness_registry();
    registry.update_arrival("cdc_topic:orders", "cdc");
    registry.update_origin("cdc_topic:orders", "cdc", commit_sec);
    ack(&event).map_err(|err| HandlerError::Ack(err.to_string()))
}

pub fn handle_chain_header<F, E>(header: ChainHeader, ack: F) -> Result<(), HandlerError>
where
    F: FnOnce(&ChainHeader) -> Result<(), E>,
    E: Error + Send + Sync + 'static,
{
    if header.height == 0 {
        return Err(HandlerError::Invalid("height must be nonzero".to_string()));
    }
    if header.hash == [0; 32] {
        return Err(HandlerError::Invalid("hash missing".to_string()));
    }
    if header.parent_hash == [0; 32] {
        return Err(HandlerError::Invalid("parent hash missing".to_string()));
    }
    let registry = global_staleness_registry();
    registry.update_arrival("chain_header", "chain");
    if let Some(origin) = header.timestamp {
        let origin_sec = epoch_seconds(origin)?;
        registry.update_origin("chain_header", "chain", origin_sec);
    }
    ack(&header).map_err(|err| HandlerError::Ack(err.to_string()))
}
