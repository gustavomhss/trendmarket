use std::env;
use std::time::{Duration, SystemTime};

use anyhow::{anyhow, Context, Result};
use credit_engine_core::obs::consumers::{
    handle_cdc_orders, handle_chain_header, handle_market_feed, handle_oracle_event, CdcOrderEvent,
    ChainHeader, MarketFeedEvent, OracleEvent, OrderChange,
};

fn main() -> Result<()> {
    let mut args = env::args().skip(1);
    let mut total = 0usize;
    while let Some(arg) = args.next() {
        if arg != "--emit" {
            return Err(anyhow!("unexpected argument {arg}"));
        }
        let source = args
            .next()
            .ok_or_else(|| anyhow!("missing source after --emit"))?;
        let count = args
            .next()
            .ok_or_else(|| anyhow!("missing count for {source}"))?
            .parse::<usize>()
            .context("count must be an integer")?;
        emit(&source, count)?;
        total += count;
    }
    println!("emitted {total} events");
    Ok(())
}

fn emit(source: &str, count: usize) -> Result<()> {
    match source {
        "oracle" => emit_oracle(count),
        "market_feed" => emit_market(count),
        "cdc_topic:orders" => emit_cdc(count),
        "chain_header" => emit_chain(count),
        other => Err(anyhow!("unsupported source {other}")),
    }
}

fn emit_oracle(count: usize) -> Result<()> {
    for index in 0..count {
        let base = 1.0 + (index as f64) * 0.0001;
        let event = OracleEvent {
            instrument: format!("FX-PAIR-{index}"),
            best_bid: base,
            best_ask: base + 0.0003,
            origin_timestamp: Some(SystemTime::now() - Duration::from_millis(40)),
        };
        handle_oracle_event(event, |_| Ok(()))?;
    }
    Ok(())
}

fn emit_market(count: usize) -> Result<()> {
    for index in 0..count {
        let event = MarketFeedEvent {
            venue: "XNAS".to_string(),
            symbol: format!("EQ-{index}"),
            last_price: 100.0 + index as f64,
            size: 10.0,
            sequence: (index + 1) as u64,
            exchange_timestamp: Some(SystemTime::now() - Duration::from_millis(30)),
        };
        handle_market_feed(event, |_| Ok(()))?;
    }
    Ok(())
}

fn emit_cdc(count: usize) -> Result<()> {
    for index in 0..count {
        let event = CdcOrderEvent {
            order_id: format!("order-{index}"),
            change: if index % 2 == 0 {
                OrderChange::Created
            } else {
                OrderChange::Updated
            },
            commit_ts: SystemTime::now() - Duration::from_secs((index % 3) as u64 + 1),
        };
        handle_cdc_orders(event, |_| Ok(()))?;
    }
    Ok(())
}

fn emit_chain(count: usize) -> Result<()> {
    let mut height = 1000u64;
    for index in 0..count {
        let hash = synth_hash(height);
        let parent = synth_hash(height - 1);
        let header = ChainHeader {
            height,
            hash,
            parent_hash: parent,
            timestamp: Some(SystemTime::now() - Duration::from_secs((index % 5) as u64 + 1)),
        };
        handle_chain_header(header, |_| Ok(()))?;
        height += 1;
    }
    Ok(())
}

fn synth_hash(input: u64) -> [u8; 32] {
    let mut seed = input ^ 0x9e3779b97f4a7c15;
    let mut bytes = [0u8; 32];
    for chunk in bytes.chunks_mut(8) {
        seed = seed.rotate_left(17) ^ 0x94d049bb133111eb;
        chunk.copy_from_slice(&seed.to_le_bytes());
    }
    bytes
}
