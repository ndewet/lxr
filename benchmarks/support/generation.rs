use criterion::{BatchSize, Criterion, SamplingMode};
use runeweaver_codegen::{RuleSpec, compile_with_modes};
use std::hint::black_box;

pub struct Input {
    pub modes: Vec<String>,
    pub rules: Vec<RuleSpec>,
}

pub struct Definition {
    name: &'static str,
    make_input: fn() -> Input,
}

impl Definition {
    pub const fn new(name: &'static str, make_input: fn() -> Input) -> Self {
        Self { name, make_input }
    }
}

pub fn benchmarks(criterion: &mut Criterion, definitions: &[Definition]) {
    let mut group = criterion.benchmark_group("generation");
    group.sampling_mode(SamplingMode::Flat);

    for definition in definitions {
        group.bench_function(definition.name, |bencher| {
            bencher.iter_batched(
                definition.make_input,
                |input| {
                    black_box(
                        compile_with_modes(black_box(input.modes), black_box(input.rules))
                            .expect("the benchmark token definition is valid"),
                    )
                },
                BatchSize::SmallInput,
            );
        });
    }

    group.finish();
}
