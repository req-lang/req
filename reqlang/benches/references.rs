use criterion::*;
use reqlang::{
    mock::generator::{ExponentialSizeIterator, Generate, ReferenceText},
    node::tree::Tree,
    renderer::formatter,
    syntax::{parser, tokenizer},
};

type Axis = (&'static str, usize, fn(&mut ReferenceText, u64));

const AXES: [Axis; 4] = [
    ("requirements", 6, |generator, size| {
        generator.requirements = size
    }),
    ("per_line", 6, |generator, size| generator.per_line = size),
    ("segments", 5, |generator, size| generator.segments = size),
    ("words", 6, |generator, size| generator.words = size),
];

fn markups(tree: &Tree) -> Vec<String> {
    (0..tree.markups.len())
        .map(|idx| {
            let mut text = Vec::new();
            formatter::markup::format(tree, idx.into(), 0, &mut text).unwrap();
            String::from_utf8(text).unwrap()
        })
        .collect()
}

fn sweep<R>(c: &mut Criterion, phase: &str, input: fn(&Tree) -> Vec<String>, run: fn(&str) -> R) {
    for (axis, sizes, set) in AXES {
        let mut group = c.benchmark_group(format!("references::{phase}::{axis}"));
        group.plot_config(PlotConfiguration::default().summary_scale(AxisScale::Logarithmic));
        for size in ExponentialSizeIterator::new().take(sizes) {
            let mut generator = ReferenceText::new();
            set(&mut generator, size);
            let texts = input(&generator.generate());

            let bytes = texts.iter().map(|text| text.len() as u64).sum();
            group.throughput(Throughput::BytesDecimal(bytes));
            group.bench_with_input(BenchmarkId::from_parameter(size), &texts, |b, texts| {
                b.iter(|| {
                    for text in texts {
                        std::hint::black_box(run(std::hint::black_box(text)));
                    }
                })
            });
        }
        group.finish();
    }
}

pub fn tokenizing(c: &mut Criterion) {
    sweep(c, "tokenizing", markups, tokenizer::markup::tokenize);
}

pub fn parsing(c: &mut Criterion) {
    let whole = |tree: &Tree| vec![formatter::format_string(tree, 0)];
    sweep(c, "parsing", whole, parser::parse);
}

criterion_group!(benches, tokenizing, parsing);
criterion_main!(benches);
