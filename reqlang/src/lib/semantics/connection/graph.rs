use crate::{
    node::{self, entity::Arena, tree::Tree},
    semantics::{
        connection::entity::{Connection, Neighbors},
        link::Links,
    },
};

pub(super) fn collect_neighbors(tree: &Tree, links: &Links, neighbors: &mut Arena<Neighbors>) {
    use node::entity::Variant::*;

    let parents = tree.entities.parents();
    for (idx, node) in tree.entities.enumerate() {
        neighbors[idx].parent = parents[idx].map(Connection::new);

        if let Requirement(req) = &node.variant {
            if let Some(reference) = req.allocation
                && let Some(alloc) = links.references[reference]
            {
                neighbors[alloc]
                    .inbound
                    .allocations
                    .push(Connection::new(idx));
                neighbors[idx].outbound.allocation = Some(Connection::new(alloc));
            }

            if let Some(reference) = req.restriction
                && let Some(restr) = links.references[reference]
            {
                neighbors[restr]
                    .inbound
                    .restrictions
                    .push(Connection::new(idx));
                neighbors[idx].outbound.restriction = Some(Connection::new(restr));
            }

            for trace in &req.traceability {
                if let Some(source) = links.references[trace.target] {
                    neighbors[source].inbound.traceability[trace.kind].push(Connection::new(idx));
                    neighbors[idx].outbound.traceability[trace.kind].push(Connection::new(source));
                }
            }

            use node::entity::RequirementVariant::*;

            match &req.variant {
                Formal(expr) => expression::collect_neighbors(tree, links, *expr, neighbors, idx),
                Informal(markup) => {
                    for reference in markup.references() {
                        if let Some(src) = links.references[reference] {
                            neighbors[src].inbound.usages.push(Connection::new(idx));
                            neighbors[idx].outbound.usages.push(Connection::new(src));
                        }
                    }
                }
            }
        }
    }
}

mod expression {
    use crate::{
        node::{self, entity::Arena, entity::Idx, tree::Tree},
        semantics::{
            connection::entity::{Connection, Neighbors},
            link::{Links, Target},
        },
    };

    pub(super) fn collect_neighbors(
        tree: &Tree,
        links: &Links,
        root: node::expression::Idx,
        neighbors: &mut Arena<Neighbors>,
        parent: Idx,
    ) {
        use node::expression::Variant::*;

        for (_, node) in tree.expressions.subtree(root) {
            if let Identifier(ident) = &node.variant
                && let Some(Target::Entity(src)) = links.identifiers[ident.target]
            {
                neighbors[src].inbound.usages.push(Connection::new(parent));
                neighbors[parent].outbound.usages.push(Connection::new(src));
            }
        }
    }
}
