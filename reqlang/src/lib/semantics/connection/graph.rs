use std::collections::HashMap;

use crate::{
    node::{self, entity::Idx},
    semantics::link,
    utils::upsert::Upsert,
};

#[derive(Default)]
pub(super) struct OutboundGraph {
    pub derivations: HashMap<Idx, Vec<Idx>>,
    pub refinements: HashMap<Idx, Vec<Idx>>,
    pub specializations: HashMap<Idx, Vec<Idx>>,
    pub restriction: HashMap<Idx, Idx>,
    pub allocation: HashMap<Idx, Idx>,
    pub usages: HashMap<Idx, Vec<Idx>>,
}

#[derive(Default)]
pub(super) struct InboundGraph {
    pub derivations: HashMap<Idx, Vec<Idx>>,
    pub refinements: HashMap<Idx, Vec<Idx>>,
    pub specializations: HashMap<Idx, Vec<Idx>>,
    pub restrictions: HashMap<Idx, Vec<Idx>>,
    pub allocations: HashMap<Idx, Vec<Idx>>,
    pub usages: HashMap<Idx, Vec<Idx>>,
}

#[derive(Default)]
pub(super) struct Graph {
    pub outbound: OutboundGraph,
    pub inbound: InboundGraph,
    pub parent: HashMap<Idx, Idx>,
}

pub(super) fn build_graph<N, E, M>(
    arena: &Vec<link::entity::Node<N, E, M>>,
    graph: &mut Graph,
    parent: Option<Idx>,
) {
    use node::entity::Variant::*;

    for (idx, node) in arena.iter().enumerate() {
        let idx = Idx::from(idx);

        if let Some(parent) = parent {
            graph.parent.insert(idx.into(), parent);
        }

        if let Requirement(req) = &node.variant {
            if let Some(alloc) = req.allocation.as_ref().and_then(|a| a.target()) {
                graph.inbound.allocations.upsert(*alloc, idx);
                graph.outbound.allocation.insert(idx, *alloc);
            }

            if let Some(restr) = req.restriction.as_ref().and_then(|a| a.target()) {
                graph.inbound.restrictions.upsert(*restr, idx);
                graph.outbound.restriction.insert(idx, *restr);
            }

            for trace in &req.traceability {
                use node::entity::TraceabilityKind::*;

                if let Some(source) = trace.target.target() {
                    let src = *source;
                    match trace.kind {
                        Specialization => graph.inbound.specializations.upsert(src, idx),
                        Refinement => graph.inbound.refinements.upsert(src, idx),
                        Derivation => graph.inbound.derivations.upsert(src, idx),
                    }
                    match trace.kind {
                        Specialization => graph.outbound.specializations.upsert(idx, src),
                        Refinement => graph.outbound.refinements.upsert(idx, src),
                        Derivation => graph.outbound.derivations.upsert(idx, src),
                    }
                }
            }

            use node::entity::RequirementVariant::*;

            match &req.variant {
                Formal(expr) => expression::build_graph(&expr, graph, idx),
                Informal(markup) => markup::build_graph(&markup, graph, idx),
            }
        }
    }
}

mod expression {
    use crate::{
        node::{self},
        semantics::{
            self,
            connection::Graph,
            link::{self},
        },
        utils::upsert::Upsert,
    };

    pub(super) fn build_graph<P>(
        arena: &Vec<link::expression::Node<P>>,
        graph: &mut Graph,
        parent: node::entity::Idx,
    ) {
        use node::expression::Variant::*;
        use semantics::link::expression::Target::*;

        for node in arena {
            if let Identifier(ident) = &node.variant
                && let Some(Entity(src)) = ident.target.target()
            {
                graph.inbound.usages.upsert(*src, parent);
                graph.outbound.usages.upsert(parent, *src);
            }
        }
    }
}

mod markup {
    use crate::{
        node::entity::Idx,
        semantics::{connection::Graph, link},
        utils::upsert::Upsert,
    };

    pub(super) fn build_graph<P>(node: &link::markup::Node<P>, graph: &mut Graph, parent: Idx) {
        for rf in node.container.references() {
            if let Some(src) = rf.target() {
                graph.inbound.usages.upsert(*src, parent);
                graph.outbound.usages.upsert(parent, *src);
            }
        }
    }
}
