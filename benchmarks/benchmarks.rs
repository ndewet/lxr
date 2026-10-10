mod cases;
mod support;

use criterion::{Criterion, criterion_group, criterion_main};
use std::time::Duration;

fn configuration() -> Criterion {
    Criterion::default()
        .confidence_level(0.99)
        .significance_level(0.01)
        .sample_size(100)
        .warm_up_time(Duration::from_millis(250))
        .measurement_time(Duration::from_millis(1_200))
        .without_plots()
}

criterion_group! {
    name = benches;
    config = configuration();
    targets = cases::benchmarks
}
criterion_main!(benches);
