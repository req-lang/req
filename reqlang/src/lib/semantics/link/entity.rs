use crate::{
    node::{
        entity::{Arena, Idx},
        path::Path,
        tree::Tree,
    },
    semantics::{
        link::{FindCtx, Links, expression},
        path::{self, Location},
    },
};

/// Fills the links of the tree, `paths` must come from the same entities
pub fn link(tree: &Tree, paths: &Arena<Location>, links: &mut Links) {
    let nodes = path::reverse(paths);
    let parents = tree.entities.parents();
    let expression_parents = tree.expressions.parents();

    for ((node, path), parent) in tree.entities.iter().zip(paths).zip(parents) {
        let imports = parent
            .and_then(|parent| tree.entities[parent].variant.imports())
            .into_iter()
            .flatten()
            .map(|import| &tree.references[*import])
            .collect();
        let current = path.path.parent().unwrap_or_default();
        let ctx = FindCtx::new(&tree.entities, &nodes, &current, imports);

        let references = node
            .variant
            .references()
            .chain(node.markups().flat_map(|markup| markup.references()));
        for reference in references {
            links.references[reference] = find(&ctx, &tree.references[reference]);
        }

        for root in node.variant.expressions() {
            expression::link(&ctx, tree, root, &expression_parents, links);
        }
    }
}

pub(super) fn find(ctx: &FindCtx, rf: &Path) -> Option<Idx> {
    if let Some(direct) = ctx.nodes.get(rf) {
        return Some(*direct);
    }

    let child = ctx.current.join(rf);
    if let Some(child) = ctx.nodes.get(&child) {
        return Some(*child);
    }

    for import in &ctx.imports {
        let imported = import.join(rf);
        if let Some(imported) = ctx.nodes.get(&imported) {
            return Some(*imported);
        }
    }

    None
}
