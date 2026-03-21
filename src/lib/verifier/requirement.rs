use std::ops::ControlFlow;

use crate::{
    diagnostic::{Issue, IssueKind},
    mapper::{id::NodeMap, path::PathMap, requirement::RequirementMap},
    syntax::{
        NodeId,
        entity::{Entity, EntityVariant, RequirementVariant},
    },
    verifier::typing::{TypeKind, TypeMap},
    visitor::{Visitor, WalkPre},
};

pub struct Verifier<'a> {
    nodes_by_id: &'a NodeMap<'a>,
    paths: &'a PathMap,
    map: &'a RequirementMap,
    cycles: &'a Vec<Vec<NodeId>>,
    types: &'a TypeMap,
    pub issues: Vec<Issue>,
}

impl Visitor<&Entity> for Verifier<'_> {
    type WalkKind = WalkPre;

    fn visit(&mut self, node: &Entity) -> ControlFlow<()> {
        use EntityVariant::*;
        use IssueKind::*;
        use RequirementVariant::*;

        if let Requirement(requirement) = &node.variant {
            let mut issues = Vec::new();

            if let Formal(expr) = &requirement.variant {
                let iss = match self.types.get(&expr.id) {
                    Some(TypeKind::Boolean) => None,
                    None => Some(MissingType(TypeKind::Boolean)),
                    Some(t) => Some(UnexpectedType(TypeKind::Boolean, t.clone())),
                };

                let iss = iss.into_iter().map(|i| Issue::new(expr.id, i));
                self.issues.extend(iss);
            }

            match self.map.on.get(&node.id) {
                None => issues.push(MissingRequirementBinding),
                Some(id) => match self.nodes_by_id.entities.get(id).unwrap().variant {
                    Part(_) | Attribute(_) => {}
                    _ => issues.push(NotPartOrAttribute(self.paths.get(id).unwrap().clone())),
                },
            }

            for cycle in self.cycles {
                if cycle.contains(&node.id) {
                    let path = cycle.iter().map(|n| self.paths.get(n).unwrap().clone());
                    issues.push(CircularTrace(path.collect()));
                }
            }

            if let Some(id) = self.map.backward.get(&node.id) {
                match self.nodes_by_id.entities.get(id).unwrap().variant {
                    Requirement(_) => {}
                    _ => issues.push(NotARequirement(self.paths.get(id).unwrap().clone())),
                }
            }

            let iss = issues.into_iter().map(|i| Issue::new(node.id, i));
            self.issues.extend(iss);
        }
        ControlFlow::Continue(())
    }
}

impl<'a> Verifier<'a> {
    pub fn new(
        nodes_by_id: &'a NodeMap<'a>,
        paths: &'a PathMap,
        map: &'a RequirementMap,
        cycles: &'a Vec<Vec<NodeId>>,
        types: &'a TypeMap,
    ) -> Self {
        Self {
            nodes_by_id,
            paths,
            map,
            cycles,
            types,
            issues: Vec::new(),
        }
    }
}
