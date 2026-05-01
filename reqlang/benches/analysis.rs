use criterion::*;
use reqlang::mock::generator::{self, Generate};

pub mod simple {

    use reqlang::{
        mock::generator::ExponentialSizeIterator,
        node::entity::Idx,
        semantics::{
            connection::{
                Connect,
                entity::{Connection, Neighbors},
            },
            link::{self, Resolve},
            path::Locate,
            typing::{Evaluate, TypeResult},
        },
        syntax,
    };

    use super::*;

    fn complete_pipeline(
        root: Vec<syntax::entity::Node>,
    ) -> Vec<link::entity::Node<Neighbors<Connection<Idx>>, TypeResult, ()>> {
        root.locate().resolve().evaluate().connect()
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
