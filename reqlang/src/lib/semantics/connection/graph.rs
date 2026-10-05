use crate::{
    node::{
        self,
        entity::{Arena, Idx},
    },
    semantics::{connection::entity::Neighbors, link::entity::Node},
};

pub(super) fn collect_neighbors<N, E, M>(
    arena: &Arena<Node<N, E, M>>,
    neighbors: &mut Arena<Neighbors<Idx>>,
) {
    use node::entity::Variant::*;

    arena
        .walk(None, |node, parent, idx| {
            neighbors[idx].parent = parent;

            if let Requirement(req) = &node.variant {
                if let Some(alloc) = req.allocation.as_ref().and_then(|a| a.target()) {
                    neighbors[*alloc].inbound.allocations.push(idx);
                    neighbors[idx].outbound.allocation = Some(*alloc);
                }

                if let Some(restr) = req.restriction.as_ref().and_then(|a| a.target()) {
                    neighbors[*restr].inbound.restrictions.push(idx);
                    neighbors[idx].outbound.restriction = Ok(*restr);
                }

                for trace in &req.traceability {
                    if let Some(source) = trace.target.target() {
                        neighbors[*source].inbound.traceability[trace.kind].push(idx);
                        neighbors[idx].outbound.traceability[trace.kind].push(*source);
                    }
                }

                use node::entity::RequirementVariant::*;

                match &req.variant {
                    Formal(expr) => expression::collect_neighbors(&expr, neighbors, idx),
                    Informal(markup) => markup::collect_neighbors(&markup, neighbors, idx),
                }
            }

            Some(idx)
        })
        .for_each(drop);
}

mod expression {
    use crate::{
        node::{self, expression::Arena},
        semantics::{self, connection::entity::Neighbors, link::expression::Node},
    };

    pub(super) fn collect_neighbors<P>(
        arena: &Arena<Node<P>>,
        neighbors: &mut node::entity::Arena<Neighbors<node::entity::Idx>>,
        parent: node::entity::Idx,
    ) {
        use node::expression::Variant::*;
        use semantics::link::Target::*;

        arena.into_iter().for_each(|node| {
            if let Identifier(ident) = &node.variant
                && let Some(tgt) = ident.target.target()
                && let Some(Entity(src)) = tgt.outer()
            {
                neighbors[*src].inbound.usages.push(parent);
                neighbors[parent].outbound.usages.push(*src);
            }
        });
    }
}

mod markup {
    use crate::{
        node::entity::{Arena, Idx},
        semantics::{connection::entity::Neighbors, link::markup::Node},
    };

    pub(super) fn collect_neighbors<P>(
        node: &Node<P>,
        graph: &mut Arena<Neighbors<Idx>>,
        parent: Idx,
    ) {
        for rf in node.container.references() {
            if let Some(src) = rf.target() {
                graph[*src].inbound.usages.push(parent);
                graph[parent].outbound.usages.push(*src);
            }
        }
    }
}
