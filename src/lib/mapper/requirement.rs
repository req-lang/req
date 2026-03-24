use std::{
    collections::{HashMap, HashSet},
    ops::ControlFlow,
};

use crate::{
    syntax::{
        entity::{Entity, EntityKey, EntityVariant, RequirementVariant},
        expression::{Expression, ExpressionVariant},
    },
    upsert::Upsert,
    verifier::reference::ReferenceMap,
    visitor::{Visitor, Walk, WalkPre},
};

pub type TraceabilityMap<'a> = HashMap<EntityKey<'a>, Vec<EntityKey<'a>>>;
pub type RestrictionMap<'a> = HashMap<EntityKey<'a>, EntityKey<'a>>;
pub type AllocationMap<'a> = HashMap<EntityKey<'a>, EntityKey<'a>>;
pub type UsageMap<'a> = HashMap<EntityKey<'a>, HashSet<EntityKey<'a>>>;

#[derive(Debug, Clone, Default)]
pub struct RequirementMap<'a> {
    pub restrictions: RestrictionMap<'a>,
    pub allocations: AllocationMap<'a>,
    pub forward: TraceabilityMap<'a>,
    pub backward: TraceabilityMap<'a>,
    pub usages: UsageMap<'a>,
}

pub struct Mapper<'a> {
    references: &'a ReferenceMap<'a>,
    current: Option<EntityKey<'a>>,
    pub map: RequirementMap<'a>,
}

impl<'a> Visitor<&'a Entity> for Mapper<'a> {
    type WalkKind = WalkPre;

    fn visit(&mut self, node: &'a Entity) -> std::ops::ControlFlow<()> {
        use EntityVariant::*;
        use RequirementVariant::*;

        if let Requirement(requirement) = &node.variant {
            if let Some(restricted) = &requirement.restriction {
                if let Some(reference) = self.references.get(&restricted.into()) {
                    self.map.restrictions.insert(node.into(), *reference);
                }
            }

            if let Some(allocated) = &requirement.allocation {
                if let Some(reference) = self.references.get(&allocated.into()) {
                    self.map.allocations.insert(node.into(), *reference);
                }
            }

            for traceability in &requirement.traceability {
                if let Some(reference) = self.references.get(&traceability.target) {
                    let source = *reference;

                    self.map.backward.upsert(node.into(), source);
                    self.map.forward.upsert(source, node.into());
                }
            }

            self.current = Some(node.into());
            match &requirement.variant {
                Formal(node) => {
                    self.walk(node)?;
                }
                Informal(container) => {
                    for reference in container.references() {
                        if let Some(id) = self.references.get(&reference.into()) {
                            self.map.usages.upsert(self.current.unwrap(), *id);
                        }
                    }
                }
            }
        }

        ControlFlow::Continue(())
    }
}

impl<'a> Visitor<&'a Expression> for Mapper<'a> {
    type WalkKind = WalkPre;

    fn visit(&mut self, node: &'a Expression) -> ControlFlow<(), ()> {
        use ExpressionVariant::*;

        if let Identifier(ident) = &node.variant {
            if let Some(id) = self.references.get(&ident.target.into()) {
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
