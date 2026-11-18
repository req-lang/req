use crate::{
    node::{self, entity::Arena, tree::Tree},
    semantics::{
        connection::entity::Neighbors,
        link::{Links, Target},
    },
};

pub(super) fn collect_neighbors(tree: &Tree, links: &Links, neighbors: &mut Arena<Neighbors>) {
    use node::entity::Variant::*;

    let parents = tree.entities.parents();
    for (idx, node) in tree.entities.enumerate() {
        neighbors[idx].parent = parents[idx];

        if let Requirement(req) = &node.variant {
            if let Some(reference) = req.allocation
                && let Some(alloc) = links.references[reference]
            {
                neighbors[alloc].inbound.allocations.push(idx);
                neighbors[idx].outbound.allocation = Some(alloc);
            }

            if let Some(reference) = req.restriction
                && let Some(restr) = links.references[reference]
            {
                neighbors[restr].inbound.restrictions.push(idx);
                neighbors[idx].outbound.restriction = Some(restr);
            }

            for trace in &req.traceability {
                if let Some(source) = links.references[trace.target] {
                    neighbors[source].inbound.traceability[trace.kind].push(idx);
                    neighbors[idx].outbound.traceability[trace.kind].push(source);
                }
            }

            use node::entity::RequirementVariant::*;

            match &req.variant {
                Formal(expr) => {
                    use node::expression::Variant::*;

                    for (_, node) in tree.expressions.subtree(*expr) {
                        if let Identifier(ident) = &node.variant
                            && let Some(Target::Entity(src)) = links.identifiers[ident.target]
                        {
                            neighbors[src].inbound.usages.push(idx);
                            neighbors[idx].outbound.usages.push(src);
                        }
                    }
                }
                Informal(markup) => {
                    for reference in tree.markups[*markup].references() {
                        if let Some(src) = links.references[reference] {
                            neighbors[src].inbound.usages.push(idx);
                            neighbors[idx].outbound.usages.push(src);
                        }
                    }
                }
            }
        }
    }
}
