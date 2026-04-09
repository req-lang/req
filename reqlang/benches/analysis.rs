use criterion::*;
use reqlang::mock::generator::{self, Generate};

pub mod simple {

    use reqlang::{
        mock::generator::ExponentialSizeIterator,
        node::{self, NodeRef},
        semantics::{connection, reference, typing},
        syntax,
    };

    use super::*;

    fn complete_pipeline(root: NodeRef<syntax::entity::Node>) -> NodeRef<connection::entity::Node> {
        let ast = node::map(root, syntax::entity::Node::from);
        let resolved = reference::resolve(ast);
        let _ = typing::evaluate(resolved.clone());
        connection::connect(resolved)
    }

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

criterion_group!(benches, simple::package, simple::depth);
criterion_main!(benches);
