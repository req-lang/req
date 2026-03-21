use std::ops::ControlFlow;

use crate::{
    diagnostic::{Issue, IssueKind},
    mapper::path::PathMap,
    syntax::entity::{Entity, EntityVariant},
    visitor::{Visitor, WalkPre},
};

pub struct Verifier<'a> {
    paths: &'a PathMap,
    pub issues: Vec<Issue>,
}

impl Visitor<&Entity> for Verifier<'_> {
    type WalkKind = WalkPre;

    fn visit(&mut self, node: &Entity) -> ControlFlow<()> {
        use EntityVariant::*;
        use IssueKind::*;

        if let Package(package) = &node.variant {
            let mut issues = Vec::new();

            for child in &package.children {
                let path = self.paths.get(&child.id).unwrap();
                if let Attribute(_) = child.variant {
                    issues.push(UnexpectedAttribute(path.clone()));
                }
            }

            let iss = issues.into_iter().map(|i| Issue::new(node.id, i));
            self.issues.extend(iss);
        }

        ControlFlow::Continue(())
    }
}

impl<'a> Verifier<'a> {
    pub fn new(paths: &'a PathMap) -> Self {
        Self {
            paths,
            issues: Vec::new(),
        }
    }
}
