pub mod issue;
pub mod mock;
pub mod node;
pub mod renderer;
pub mod semantics;
pub mod syntax;
pub mod utils;

#[derive(Debug, Clone)]
pub struct Specification {
    pub tree: node::tree::Tree,
    pub analysis: semantics::Analysis,
}

impl From<node::tree::Tree> for Specification {
    fn from(tree: node::tree::Tree) -> Self {
        let analysis = semantics::Analysis::from(&tree);
        Specification { tree, analysis }
    }
}

impl Specification {
    pub fn diagnose(&mut self) {
        self.analysis.diagnose(&self.tree);
    }
}
