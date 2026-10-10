use criterion::{BatchSize, Criterion};
use runeweaver_codegen::{RuleSpec, compile};
use std::hint::black_box;

pub struct Definition {
    name: &'static str,
    make_rules: fn() -> Vec<RuleSpec>,
}

impl Definition {
    pub const fn new(name: &'static str, make_rules: fn() -> Vec<RuleSpec>) -> Self {
        Self { name, make_rules }
    }
}

pub fn benchmarks(criterion: &mut Criterion, definitions: &[Definition]) {
    let mut group = criterion.benchmark_group("generation");

    for definition in definitions {
        group.bench_function(definition.name, |bencher| {
            bencher.iter_batched(
                definition.make_rules,
                |rules| {
                    black_box(
                        compile(black_box(rules)).expect("the benchmark token definition is valid"),
                    )
                },
                BatchSize::SmallInput,
            );
        });
    }

    group.finish();
}
