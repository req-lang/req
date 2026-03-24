use std::ops::ControlFlow;

use crate::{
    diagnostic::{Issue, IssueKind},
    mapper::{path::PathMap, subject::SubjectMap},
    syntax::entity::{Entity, EntityVariant},
    visitor::{Visitor, WalkPre},
};

pub struct Verifier<'a> {
    smap: &'a SubjectMap<'a>,
    paths: &'a PathMap<'a>,
    pub issues: Vec<Issue<'a>>,
}

impl Visitor<&Entity> for Verifier<'_> {
    type WalkKind = WalkPre;

    fn visit(&mut self, node: &Entity) -> ControlFlow<()> {
        use EntityVariant::*;
        use IssueKind::*;

        if let Part(part) = &node.variant {
            let mut issues = Vec::new();

            for child in &part.children {
                match &child.variant {
                    Part(_) | Attribute(_) => {}
                    _ => issues.push(NotPartOrAttribute(child.into())),
                }
            }

            if let None = self.smap.usages.get(&node.into()) {
                issues.push(Unused);
            }

            if let None = self.smap.restrictions.get(&node.into())
                && part.children.is_empty()
            {
                issues.push(Unrestricted);
            }

            if let None = node.meta.comment {
                issues.push(Uncommented);
            }

            let iss = issues.into_iter().map(|i| Issue::new(node.into(), i));
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
