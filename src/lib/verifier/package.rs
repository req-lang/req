use std::ops::ControlFlow;

use crate::{
    issue::entity::{Issue, IssueKind},
    mapper::path::PathMap,
    syntax::entity::{Entity, EntityVariant},
    visitor::{Visitor, WalkPre},
};

pub struct Verifier<'a> {
    paths: &'a PathMap<'a>,
    pub issues: Vec<Issue<'a>>,
}

impl<'n> Visitor<&'n Entity> for Verifier<'n> {
    type WalkKind = WalkPre;

    fn visit(&mut self, node: &'n Entity) -> ControlFlow<()> {
        use EntityVariant::*;
        use IssueKind::*;

        if let Package(package) = &node.variant {
            let mut issues = Vec::new();

            if package.children.len() == 0 {
                issues.push(EmptyPackage);
            }

            for child in &package.children {
                if let Attribute(_) = child.variant {
                    issues.push(UnexpectedAttribute(child.into()));
                }
            }

            let iss = issues.into_iter().map(|i| Issue::new(node.into(), i));
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
