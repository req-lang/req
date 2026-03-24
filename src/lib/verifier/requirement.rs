use std::{collections::HashMap, ops::ControlFlow};

use crate::{
    diagnostic::{Issue, IssueKind},
    mapper::{ancestors::AncestorsMap, id::NodeMap, path::PathMap, requirement::RequirementMap},
    syntax::entity::{
        Entity, EntityKey, EntityVariant, RequirementVariant, TraceabilityInhibitor,
        TraceabilityKind, TraceabilityLink,
    },
    verifier::typing::{TypeKind, TypeMap},
    visitor::{Visitor, WalkPre},
};

pub struct Verifier<'a> {
    nodes_by_id: &'a NodeMap<'a>,
    paths: &'a PathMap<'a>,
    map: &'a RequirementMap<'a>,
    trace_cycles: &'a Vec<Vec<EntityKey<'a>>>,
    types: &'a TypeMap<'a>,
    ancestors: &'a AncestorsMap<'a>,
    pub issues: Vec<Issue<'a>>,
}

impl Visitor<&Entity> for Verifier<'_> {
    type WalkKind = WalkPre;

    // TODO: Detect circular allocations
    fn visit(&mut self, node: &Entity) -> ControlFlow<()> {
        use EntityVariant::*;
        use IssueKind::*;
        use RequirementVariant::*;

        if let Requirement(requirement) = &node.variant {
            let mut issues = Vec::new();
            let forward = self.map.forward.get(&node.into());
            let backward = self.map.backward.get(&node.into());

            if let Formal(expr) = &requirement.variant {
                let iss = match self.types.get(&expr.into()) {
                    Some(TypeKind::Boolean) | None => None,
                    Some(t) => Some(UnexpectedType(TypeKind::Boolean, t.clone())),
                };

                let iss = iss.into_iter().map(|i| Issue::new(expr.into(), i));
                self.issues.extend(iss);
            }

            match self.map.restrictions.get(&node.into()) {
                None => issues.push(MissingRequirementBinding),
                Some(id) => match self.nodes_by_id.entities.get(id).unwrap().variant {
                    Part(_) | Attribute(_) => {}
                    _ => issues.push(NotPartOrAttribute(id.into())),
                },
            }

            if let Some(vid) = self.map.allocations.get(&node.into()) {
                match self.nodes_by_id.entities.get(vid).unwrap().variant {
                    Part(_) | Attribute(_) => {}
                    _ => issues.push(NotPartOrAttribute(vid.into())),
                };

                match requirement.inhibitor {
                    Some(TraceabilityInhibitor::Isolated) | Some(TraceabilityInhibitor::Bottom) => {
                        issues.push(InhibitorForbidDerives)
                    }
                    _ => {}
                }

                match self.map.forward.get(&node.into()) {
                    Some(fids) => {
                        let derived = fids.iter().find(|fid| {
                            let forward = self.nodes_by_id.entities.get(*fid).unwrap();
                            match self.map.restrictions.get(&forward.into()) {
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

                if let Some(oid) = self.map.restrictions.get(&node.into())
                    && oid == vid
                {
                    issues.push(SameAllocationAndConstraint);
                }
            }

            for cycle in self.trace_cycles {
                if cycle.contains(&node.into()) {
                    let path = cycle.iter().map(|n| self.paths.get(n).unwrap().clone());
                    issues.push(CircularTrace(path.collect()));
                }
            }

            match requirement.inhibitor {
                None => {
                    if let None = forward {
                        issues.push(MissingForwardTrace)
                    }

                    if let None = backward {
                        issues.push(MissingBackwardTrace)
                    }
                }

                Some(TraceabilityInhibitor::Top) => {
                    if let None = forward {
                        issues.push(MissingForwardTrace)
                    }

                    if let Some(_) = backward {
                        issues.push(UnexpectedBrackwardTrace)
                    }
                }

                Some(TraceabilityInhibitor::Bottom) => {
                    if let Some(_) = forward {
                        issues.push(UnexpectedForwardTrace)
                    }

                    if let None = backward {
                        issues.push(MissingBackwardTrace)
                    }
                }

                Some(TraceabilityInhibitor::Isolated) => {
                    if let Some(_) = forward {
                        issues.push(UnexpectedForwardTrace)
                    }

                    if let Some(_) = backward {
                        issues.push(UnexpectedBrackwardTrace)
                    }
                }
            }

            if let Some(ids) = self.map.backward.get(&node.into()) {
                use TraceabilityKind::*;

                let mut seen: HashMap<&EntityKey, &TraceabilityLink> = HashMap::new();
                for (id, trace) in ids.iter().zip(&requirement.traceability) {
                    match seen.get(id) {
                        Some(first) => match &first.kind {
                            k if *k == trace.kind => issues.push(DuplicatedTrace(id.into())),
                            _ => issues.push(ConflictingTrace(id.into())),
                        },
                        None => {
                            seen.insert(id, trace);
                        }
                    }

                    let back = self.nodes_by_id.entities.get(id).unwrap();
                    match back.variant {
                        Requirement(_) => {}
                        _ => issues.push(NotARequirement(id.into())),
                    }

                    match (
                        self.map.restrictions.get(&node.into()),
                        self.map.restrictions.get(&back.into()),
                    ) {
                        (Some(nid), Some(bid)) => match &trace.kind {
                            Specialization if nid != bid => {
                                issues.push(SpecializationMismatch(bid.into()));
                            }

                            Refinement if nid == bid => {
                                issues.push(PossibleDuplicate(bid.into()));
                            }

                            Refinement
                                if nid != bid
                                    && !self.ancestors.get(nid).unwrap().contains(bid) =>
                            {
                                issues.push(RefinementMismatch(bid.clone()));
                            }
                            _ => {}
                        },
                        _ => {}
                    }

                    if let Some(nid) = self.map.restrictions.get(&node.into())
                        && trace.kind == Derivation
                    {
                        match self.map.allocations.get(&back.into()) {
                            Some(vid) if vid != nid => issues.push(DerivationMismatch(back.into())),
                            None => issues.push(NoDerivedAllocation(back.into())),
                            _ => {}
                        }
                    }
                }
            }

            let iss = issues.into_iter().map(|i| Issue::new(node.into(), i));
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
        trace_cycles: &'a Vec<Vec<EntityKey<'a>>>,
        types: &'a TypeMap,
        ancestors: &'a AncestorsMap,
    ) -> Self {
        Self {
            nodes_by_id,
            paths,
            map,
            trace_cycles,
            types,
            ancestors,
            issues: Vec::new(),
        }
    }
}
