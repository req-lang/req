use crate::{
    node::{expression, path::Path, tree::Tree},
    semantics::link::{FindCtx, Links, Target},
};

/// Resolves the identifiers of the expression rooted at `root`
pub(super) fn link(
    ctx: &FindCtx,
    tree: &Tree,
    root: expression::Idx,
    parents: &expression::Arena<Option<expression::Idx>>,
    links: &mut Links,
) {
    for (idx, node) in tree.expressions.subtree(root) {
        if let expression::Variant::Identifier(ident) = &node.variant {
            let path = &tree.identifiers[ident.target];
            links.identifiers[ident.target] =
                Target::find(ctx, path).or_else(|| local(&tree.expressions, parents, idx, path));
        }
    }
}

/// The nearest set element above the expression that binds the variable named by the path
fn local(
    expressions: &expression::Arena<expression::Node>,
    parents: &expression::Arena<Option<expression::Idx>>,
    from: expression::Idx,
    path: &Path,
) -> Option<Target> {
    let mut current = Some(from);
    while let Some(idx) = current {
        if let Some(set) = expressions[idx].variant.set_element()
            && set.variable == path.as_str()
        {
            return Some(Target::Local(idx));
        }
        current = parents[idx];
    }

    None
}
