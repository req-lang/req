use std::ops::ControlFlow;

use crate::{
    diagnostic::{Issue, IssueKind},
    mapper::{path::PathMap, subject::SubjectMap},
    syntax::entity::{Entity, EntityVariant},
    visitor::{Visitor, WalkPre},
};

pub struct Verifier<'a> {
    smap: &'a SubjectMap,
    paths: &'a PathMap,
    pub issues: Vec<Issue>,
}

impl Visitor<&Entity> for Verifier<'_> {
    type WalkKind = WalkPre;

    fn visit(&mut self, node: &Entity) -> ControlFlow<()> {
        use EntityVariant::*;
        use IssueKind::*;

        if let Part(part) = &node.variant {
            let mut issues = Vec::new();

            for child in &part.children {
                let path = self.paths.get(&child.id).unwrap();
                match &child.variant {
                    Part(_) | Attribute(_) => {}
                    _ => issues.push(NotPartOrAttribute(path.clone())),
                }
            }

            if let None = self.smap.usages.get(&node.id) {
                issues.push(Unused);
            }

            if let None = self.smap.restrictions.get(&node.id) {
                issues.push(Unrestricted);
            }

            if let None = node.meta.comment {
                issues.push(Uncommented);
            }

            let iss = issues.into_iter().map(|i| Issue::new(node.id, i));
            self.issues.extend(iss);
        }

        ControlFlow::Continue(())
    }
}

impl<'a> Verifier<'a> {
    pub fn new(smap: &'a SubjectMap, paths: &'a PathMap) -> Self {
        Self {
            smap,
            paths,
            issues: Vec::new(),
        }
    }
}
