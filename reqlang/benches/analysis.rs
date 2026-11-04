use criterion::*;
use reqlang::{
    Specification,
    mock::generator::{self, Generate},
    node::tree::Tree,
};

fn complete_pipeline(tree: Tree) -> Specification {
    let mut analysis = Specification::from(tree);
    analysis.diagnose();
    analysis
}

pub mod simple {

    use reqlang::mock::generator::ExponentialSizeIterator;

    use super::*;
    pub fn package(c: &mut Criterion) {
        let mut group = c.benchmark_group("analysis::simple::package");
        group.plot_config(PlotConfiguration::default().summary_scale(AxisScale::Logarithmic));
        for size in ExponentialSizeIterator::new().take(7) {
            let mut generator = generator::Simple::new();
            generator.packages = size;

            let id = BenchmarkId::from_parameter(size);
            group.throughput(Throughput::Elements(generator.size()));
            group.bench_function(id, |b| {
                b.iter_batched(
                    || generator.generate(),
                    complete_pipeline,
                    BatchSize::SmallInput,
                );
            });
        }
        group.finish();
    }

    pub fn depth(c: &mut Criterion) {
        let mut group = c.benchmark_group("analysis::simple::depth");
        group.plot_config(PlotConfiguration::default().summary_scale(AxisScale::Logarithmic));
        for size in ExponentialSizeIterator::new().take(5) {
            let mut generator = generator::Simple::new();
            generator.depth = size;

            let id = BenchmarkId::from_parameter(size);
            group.throughput(Throughput::Elements(generator.size()));
            group.bench_function(id, |b| {
                b.iter_batched(
                    || generator.generate(),
                    complete_pipeline,
                    BatchSize::SmallInput,
                );
            });
        }
        group.finish();
    }
}

pub mod expression {
    use reqlang::mock::generator::{ExponentialSizeIterator, ExpressionTyping};

    use super::*;

    pub fn cycles(c: &mut Criterion) {
        let mut group = c.benchmark_group("analysis::expression_typing::cycles");
        group.plot_config(PlotConfiguration::default().summary_scale(AxisScale::Logarithmic));
        for size in ExponentialSizeIterator::new().take(7) {
            let generator = ExpressionTyping::new(size);

            let id = BenchmarkId::from_parameter(size);
            group.throughput(Throughput::Elements(generator.size()));
            group.bench_function(id, |b| {
                b.iter_batched(
                    || generator.generate(),
                    complete_pipeline,
                    BatchSize::SmallInput,
                );
            });
        }
        group.finish();
    }
}

pub mod reference {
    use reqlang::mock::generator::{ExponentialSizeIterator, ReferenceResolution};

    use super::*;

    pub fn requirements(c: &mut Criterion) {
        let mut group = c.benchmark_group("analysis::reference_resolution::requirements");
        group.plot_config(PlotConfiguration::default().summary_scale(AxisScale::Logarithmic));
        for size in ExponentialSizeIterator::new().take(7) {
            let generator = ReferenceResolution::new(size);

            let id = BenchmarkId::from_parameter(size);
            group.throughput(Throughput::Elements(generator.size()));
            group.bench_function(id, |b| {
                b.iter_batched(
                    || generator.generate(),
                    complete_pipeline,
                    BatchSize::SmallInput,
                );
            });
        }
        group.finish();
    }
}

criterion_group!(
    benches,
    simple::package,
    simple::depth,
    expression::cycles,
    reference::requirements
);
criterion_main!(benches);
