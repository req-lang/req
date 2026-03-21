use std::{
    collections::{HashMap, HashSet},
    ops::ControlFlow,
};

use crate::{
    syntax::{
        NodeId,
        entity::{Entity, EntityVariant, Reference, RequirementVariant},
        expression::{Expression, ExpressionVariant},
    },
    verifier::reference::ReferenceMap,
    visitor::{Visitor, Walk, WalkPre},
};

pub type BackwardMap = HashMap<NodeId, NodeId>;
pub type ForwardMap = HashMap<NodeId, Vec<NodeId>>;
pub type ConstraintMap = HashMap<NodeId, NodeId>;
pub type AllocationMap = HashMap<NodeId, NodeId>;
pub type UsageMap = HashMap<NodeId, HashSet<NodeId>>;

#[derive(Debug, Clone, Default)]
pub struct RequirementMap {
    pub on: ConstraintMap,
    pub via: AllocationMap,
    pub forward: ForwardMap,
    pub backward: BackwardMap,
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
            if let Some(constrained) = &requirement.constrained {
                if let Some(reference) = self.references.get(&constrained.rid) {
                    self.map.on.insert(node.id, *reference);
                }
            }

            if let Some(allocated) = &requirement.constrained {
                if let Some(reference) = self.references.get(&allocated.rid) {
                    self.map.via.insert(node.id, *reference);
                }
            }

            for traceability in &requirement.traceability {
                if let Some(reference) = self.references.get(&traceability.target.rid) {
                    let source = *reference;

                    self.map.backward.insert(node.id, source);

                    match self.map.forward.get_mut(reference) {
                        Some(traces) => {
                            traces.push(node.id);
                        }
                        None => {
                            self.map.forward.insert(source, vec![node.id]);
                        }
                    }
                }
            }

            self.current = Some(node.id);
            match &requirement.variant {
                Formal(node) => {
                    self.walk(node)?;
                }
                Informal(container) => {
                    for reference in container.references() {
                        self.insert(reference);
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
            self.insert(&ident.target);
        }

        ControlFlow::Continue(())
    }
}

impl Mapper<'_> {
    pub fn insert(&mut self, reference: &Reference) {
        let requirement = self.current.unwrap();
        if let Some(id) = self.references.get(&reference.rid) {
            match self.map.usages.get_mut(id) {
                Some(usages) => {
                    usages.insert(requirement);
                }
                None => {
                    let mut set = HashSet::new();
                    set.insert(requirement);
                    self.map.usages.insert(*id, set);
                }
            }
        }
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
