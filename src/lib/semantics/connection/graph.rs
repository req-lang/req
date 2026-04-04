use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::{
    node::{self, NodeRef},
    semantics::{connection::entity::Node, reference::WeakNodeRef},
    utils::{children::ChildrenIter, upsert::Upsert},
};

#[derive(Default)]
pub(super) struct OutboundGraph {
    pub derivations: HashMap<*const RefCell<Node>, Vec<WeakNodeRef<Node>>>,
    pub refinements: HashMap<*const RefCell<Node>, Vec<WeakNodeRef<Node>>>,
    pub specializations: HashMap<*const RefCell<Node>, Vec<WeakNodeRef<Node>>>,
    pub restriction: HashMap<*const RefCell<Node>, WeakNodeRef<Node>>,
    pub allocation: HashMap<*const RefCell<Node>, WeakNodeRef<Node>>,
    pub usages: HashMap<*const RefCell<Node>, Vec<WeakNodeRef<Node>>>,
}

#[derive(Default)]
pub(super) struct InboundGraph {
    pub derivations: HashMap<*const RefCell<Node>, Vec<WeakNodeRef<Node>>>,
    pub refinements: HashMap<*const RefCell<Node>, Vec<WeakNodeRef<Node>>>,
    pub specializations: HashMap<*const RefCell<Node>, Vec<WeakNodeRef<Node>>>,
    pub restrictions: HashMap<*const RefCell<Node>, Vec<WeakNodeRef<Node>>>,
    pub allocations: HashMap<*const RefCell<Node>, Vec<WeakNodeRef<Node>>>,
    pub usages: HashMap<*const RefCell<Node>, Vec<WeakNodeRef<Node>>>,
}

#[derive(Default)]
pub(super) struct Graph {
    pub outbound: OutboundGraph,
    pub inbound: InboundGraph,
    pub parent: HashMap<*const RefCell<Node>, WeakNodeRef<Node>>,
}

pub(super) fn build_graph(
    node: NodeRef<Node>,
    graph: &mut Graph,
    parent: Option<WeakNodeRef<Node>>,
) {
    use node::entity::Variant::*;

    let ptr = Rc::as_ptr(&node);
    let weak = Rc::downgrade(&node);

    if let Some(parent) = parent {
        graph.parent.insert(ptr, parent);
    }

    if let Requirement(req) = &node.borrow().variant {
        if let Some(Ok(alloc)) = &req.allocation {
            graph
                .inbound
                .allocations
                .upsert(alloc.as_ptr(), weak.clone());
            graph.outbound.allocation.insert(ptr, alloc.clone());
        }

        if let Some(Ok(restr)) = &req.restriction {
            graph
                .inbound
                .restrictions
                .upsert(restr.as_ptr(), weak.clone());
            graph.outbound.restriction.insert(ptr, restr.clone());
        }

        for trace in &req.traceability {
            use node::entity::TraceabilityKind::*;

            if let Ok(source) = &trace.target {
                let src = source.as_ptr();
                match trace.kind {
                    Specialization => graph.inbound.specializations.upsert(src, weak.clone()),
                    Refinement => graph.inbound.refinements.upsert(src, weak.clone()),
                    Derivation => graph.inbound.derivations.upsert(src, weak.clone()),
                }
                match trace.kind {
                    Specialization => graph.outbound.specializations.upsert(ptr, source.clone()),
                    Refinement => graph.outbound.refinements.upsert(ptr, source.clone()),
                    Derivation => graph.outbound.derivations.upsert(ptr, source.clone()),
                }
            }
        }

        use node::entity::RequirementVariant::*;

        let rf = Rc::downgrade(&node);
        match &req.variant {
            Formal(expr) => expression::build_graph(expr.clone(), graph, &rf),
            Informal(markup) => markup::build_graph(&markup, graph, &rf),
        }
    }

    for child in node.borrow().variant.children_iter() {
        build_graph(child, graph, Some(weak.clone()));
    }
}

mod expression {
    use crate::{
        node::{self, NodeRef},
        semantics::{
            self,
            connection::{Graph, entity, expression::Node},
            reference::WeakNodeRef,
        },
        utils::{children::ChildrenIter, upsert::Upsert},
    };

    pub(super) fn build_graph(
        node: NodeRef<Node>,
        graph: &mut Graph,
        parent: &WeakNodeRef<entity::Node>,
    ) {
        use node::expression::Variant::*;
        use semantics::reference::expression::Target::*;

        let ptr = parent.as_ptr();

        if let Identifier(ident) = &node.borrow().variant {
            if let Ok(Entity(src)) = &ident.target {
                // SAFETY: into_raw transfers ownership; from_raw in connect reclaims it
                graph.inbound.usages.upsert(src.as_ptr(), parent.clone());
                graph.outbound.usages.upsert(ptr, src.clone());
            }
        }

        for child in node.borrow().variant.children_iter() {
            build_graph(child, graph, parent);
        }
    }
}

mod markup {
    use crate::{
        semantics::{
            connection::{Graph, entity, markup::Node},
            reference::WeakNodeRef,
        },
        utils::upsert::Upsert,
    };

    pub(super) fn build_graph(node: &Node, graph: &mut Graph, parent: &WeakNodeRef<entity::Node>) {
        let ptr = parent.as_ptr();

        for rf in node.references() {
            if let Ok(src) = rf {
                // SAFETY: into_raw transfers ownership; from_raw in connect reclaims it
                graph.inbound.usages.upsert(src.as_ptr(), parent.clone());
                graph.outbound.usages.upsert(ptr, src.clone());
            }
        }
    }
}
