use std::ops::ControlFlow;

use crate::{
    diagnostic::{Issue, IssueKind},
    mapper::{ancestors::AncestorsMap, id::NodeMap, path::PathMap, requirement::RequirementMap},
    syntax::{
        NodeId,
        entity::{Entity, EntityVariant, RequirementVariant, TraceabilityKind},
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
    ancestors: &'a AncestorsMap,
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

            if let Some(vid) = self.map.via.get(&node.id) {
                match self.nodes_by_id.entities.get(vid).unwrap().variant {
                    Part(_) | Attribute(_) => {}
                    _ => issues.push(NotPartOrAttribute(self.paths.get(vid).unwrap().clone())),
                };

                match self.map.forward.get(&node.id) {
                    Some(fids) => {
                        let derived = fids.iter().find(|fid| {
                            let forward = self.nodes_by_id.entities.get(*fid).unwrap();
                            match self.map.on.get(&forward.id) {
                                Some(oid) => oid == vid,
                                None => false,
                            }
                        });
                        if derived.is_none() {
                            issues.push(MissingDerivation);
                        }
                    }
                    None => issues.push(MissingDerivation),
                }

                if let Some(oid) = self.map.on.get(&node.id)
                    && oid == vid
                {
                    issues.push(SameAllocationAndConstraint);
                }
            }

            for cycle in self.cycles {
                if cycle.contains(&node.id) {
                    let path = cycle.iter().map(|n| self.paths.get(n).unwrap().clone());
                    issues.push(CircularTrace(path.collect()));
                }
            }

            if let Some(ids) = self.map.backward.get(&node.id) {
                use TraceabilityKind::*;

                for (id, trace) in ids.iter().zip(&requirement.traceability) {
                    let back = self.nodes_by_id.entities.get(id).unwrap();
                    match back.variant {
                        Requirement(_) => {}
                        _ => issues.push(NotARequirement(self.paths.get(id).unwrap().clone())),
                    }

                    match (self.map.on.get(&node.id), self.map.on.get(&back.id)) {
                        (Some(nid), Some(bid)) => match &trace.kind {
                            Specialization if nid != bid => {
                                issues.push(SpecializationMismatch);
                            }
                            Refinement if !self.ancestors.get(nid).unwrap().contains(bid) => {
                                issues.push(RefinementMismatch);
                            }
                            Derivation => match self.map.via.get(&back.id) {
                                Some(vid) if vid != nid => issues.push(DerivationMismatch),
                                None => issues.push(NoDerivedAllocation),
                                _ => {}
                            },
                            _ => {}
                        },
                        _ => {}
                    }
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
        ancestors: &'a AncestorsMap,
    ) -> Self {
        Self {
            nodes_by_id,
            paths,
            map,
            cycles,
            types,
            ancestors,
            issues: Vec::new(),
        }
    }
}
