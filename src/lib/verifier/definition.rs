use std::{collections::HashMap, ops::ControlFlow};

use crate::{
    issue::entity::{Issue, IssueKind},
    mapper::path::PathMap,
    path::Path,
    syntax::entity::Entity,
    visitor::{Visitor, WalkPre},
};

pub struct Verifier<'n> {
    paths: &'n PathMap<'n>,
    nodes_by_path: HashMap<&'n Path, &'n Entity>,
    pub diagnostics: Vec<Issue<'n>>,
}

impl<'n> Visitor<&'n Entity> for Verifier<'n> {
    type WalkKind = WalkPre;

    fn visit(&mut self, node: &'n Entity) -> ControlFlow<()> {
        use IssueKind::*;

        let path = self.paths.get(&node.into()).unwrap();

        if let Some(duplicate) = self.nodes_by_path.insert(&path, node) {
            let err = Issue::new(node.into(), RedefinedEntity(duplicate.into()));
            self.diagnostics.push(err);
        }

        ControlFlow::Continue(())
    }
}

impl<'n> Verifier<'n> {
    pub fn new(paths: &'n PathMap<'n>) -> Self {
        Self {
            paths,
            diagnostics: Vec::default(),
            nodes_by_path: HashMap::default(),
        }
    }
}

pub type PathNodeMap<'n> = HashMap<&'n Path, &'n Entity>;
