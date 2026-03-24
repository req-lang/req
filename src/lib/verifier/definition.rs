use std::{collections::HashMap, ops::ControlFlow};

use crate::{
    diagnostic::{Issue, IssueKind},
    mapper::path::PathMap,
    path::Path,
    syntax::entity::{Entity, EntityKey},
    visitor::{Visitor, WalkPre},
};

pub struct Verifier<'a> {
    paths: &'a PathMap<'a>,
    nodes_by_path: HashMap<&'a Path, &'a Entity>,
    pub diagnostics: Vec<Issue<'a>>,
}

impl<'a> Visitor<&'a Entity> for Verifier<'a> {
    type WalkKind = WalkPre;

    fn visit(&mut self, node: &'a Entity) -> ControlFlow<()> {
        use IssueKind::*;

        let path = self.paths.get(&EntityKey(node)).unwrap();

        if let Some(duplicate) = self.nodes_by_path.insert(&path, node) {
            let err = Issue::new(EntityKey(node), RedefinedEntity(EntityKey(duplicate)));
            self.diagnostics.push(err);
        }

        ControlFlow::Continue(())
    }
}

impl<'a> Verifier<'a> {
    pub fn new(paths: &'a PathMap) -> Self {
        Self {
            paths,
            diagnostics: Vec::default(),
            nodes_by_path: HashMap::default(),
        }
    }
}

pub type PathNodeMap<'a> = HashMap<&'a Path, &'a Entity>;
