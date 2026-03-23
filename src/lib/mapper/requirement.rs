use std::{
    collections::{HashMap, HashSet},
    ops::ControlFlow,
};

use serde::Serialize;

use crate::{
    syntax::{
        NodeId,
        entity::{Entity, EntityVariant, RequirementVariant},
        expression::{Expression, ExpressionVariant},
    },
    upsert::Upsert,
    verifier::reference::ReferenceMap,
    visitor::{Visitor, Walk, WalkPre},
};

pub type TraceabilityMap = HashMap<NodeId, Vec<NodeId>>;
pub type RestrictionMap = HashMap<NodeId, NodeId>;
pub type AllocationMap = HashMap<NodeId, NodeId>;
pub type UsageMap = HashMap<NodeId, HashSet<NodeId>>;

#[derive(Debug, Clone, Default, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts", ts(export))]
pub struct RequirementMap {
    pub restrictions: RestrictionMap,
    pub allocations: AllocationMap,
    pub forward: TraceabilityMap,
    pub backward: TraceabilityMap,
    pub usages: UsageMap,
}

pub struct Mapper<'a> {
    references: &'a ReferenceMap,
    current: Option<NodeId>,
    pub map: RequirementMap,
}

impl Visitor<&Entity> for Mapper<'_> {
    type WalkKind = WalkPre;

    fn visit(&mut self, node: &Entity) -> std::ops::ControlFlow<()> {
        use EntityVariant::*;
        use RequirementVariant::*;

        if let Requirement(requirement) = &node.variant {
            if let Some(restricted) = &requirement.restriction {
                if let Some(reference) = self.references.get(&restricted.rid) {
                    self.map.restrictions.insert(node.id, *reference);
                }
            }

            if let Some(allocated) = &requirement.allocation {
                if let Some(reference) = self.references.get(&allocated.rid) {
                    self.map.allocations.insert(node.id, *reference);
                }
            }

            for traceability in &requirement.traceability {
                if let Some(reference) = self.references.get(&traceability.target.rid) {
                    let source = *reference;

                    self.map.backward.upsert(node.id, source);
                    self.map.forward.upsert(source, node.id);
                }
            }

            self.current = Some(node.id);
            match &requirement.variant {
                Formal(node) => {
                    self.walk(node)?;
                }
                Informal(container) => {
                    for reference in container.references() {
                        if let Some(id) = self.references.get(&reference.rid) {
                            self.map.usages.upsert(self.current.unwrap(), *id);
                        }
                    }
                }
            }
        }

        ControlFlow::Continue(())
    }
}

impl Visitor<&Expression> for Mapper<'_> {
    type WalkKind = WalkPre;

    fn visit(&mut self, node: &Expression) -> ControlFlow<(), ()> {
        use ExpressionVariant::*;

        if let Identifier(ident) = &node.variant {
            if let Some(id) = self.references.get(&ident.target.rid) {
                self.map.usages.upsert(self.current.unwrap(), *id);
            }
        }

        ControlFlow::Continue(())
    }
}

impl<'a> Mapper<'a> {
    pub fn new(references: &'a ReferenceMap) -> Self {
        Self {
            references,
            current: None,
            map: RequirementMap::default(),
        }
    }
}
