use std::time::Duration;
use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use credit_engine_core::amm::swap::get_amount_out; // se sua função estiver em cpmm, troque swap->cpmm
use credit_engine_core::amm::types::{Wad, WAD};


#[inline] fn w(n: u128) -> Wad { n * WAD }


fn bench_swap(c: &mut Criterion) {
let mut g = c.benchmark_group("swap");
g.warm_up_time(Duration::from_secs(2));
g.measurement_time(Duration::from_secs(5));
g.sample_size(300);
g.throughput(Throughput::Elements(1));


// Casos com rótulo único + taxa
let cases: [(&str, Wad, Wad, Wad, u32); 6] = [
("sym_small", w(1_000_000), w(1_000_000), w(1_000), 0u32),
("sym_large", w(5_000_000_000), w(5_000_000_000), w(1_000_000), 0u32),
("asym_xgg", w(1_000_000_000), w(1_000_000), w(1_000), 0u32),
("asym_ygg", w(1_000_000), w(1_000_000_000), w(1_000), 0u32),
("sym_small_fee", w(1_000_000), w(1_000_000), w(1_000), 300u32),
("asym_xgg_fee", w(1_000_000_000), w(1_000_000), w(1_000), 300u32),
];


for (label, x, y, dx, fee) in cases {
let name = format!("{}_f{}", label, fee);
g.bench_function(name, |b| {
b.iter(|| {
let dy = get_amount_out(black_box(x), black_box(y), black_box(dx), black_box(fee)).unwrap();
black_box(dy);
});
});
}
g.finish();
}


criterion_group!(benches, bench_swap);
criterion_main!(benches);
