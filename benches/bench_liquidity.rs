use std::time::Duration;
use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use credit_engine_core::amm::liquidity::remove_liquidity;
use credit_engine_core::amm::types::{Wad, WAD};
#[inline] fn w(n: u128) -> Wad { n * WAD }


fn bench_liquidity(c: &mut Criterion) {
let mut g = c.benchmark_group("liquidity");
g.warm_up_time(Duration::from_secs(2));
g.measurement_time(Duration::from_secs(5));
g.sample_size(300);
g.throughput(Throughput::Elements(1));


let (x, y) = (w(2_000_000), w(3_000_000));
let liq_all = w(1_000_000);


g.bench_function("remove_liquidity_partial", |b| {
b.iter(|| {
let (dx, dy): (Wad, Wad) = remove_liquidity(
black_box(x), black_box(y), black_box(liq_all/2), black_box(liq_all)
).expect("remove ok");
black_box((dx, dy));
});
});


g.finish();
}
criterion_group!(benches, bench_liquidity);
criterion_main!(benches);
