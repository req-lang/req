use reqlang::Specification;

pub mod entity;
pub mod expression;
pub mod markup;

pub fn render<W: std::io::Write>(spec: &Specification, out: &mut W) -> std::io::Result<()> {
    let tree = &spec.tree;
    // An empty source, or one the parser understood nothing of, has no entity at all
    if tree.entities.is_empty() {
        return Ok(());
    }
    // INVARIANT: Otherwise the root is the package of the file.
    let parents = tree.entities.parents();
    for (idx, _) in tree.entities.subtree(tree.root()) {
        // 1 for the root
        let level = std::iter::successors(Some(idx), |i| parents[*i]).count();
        entity::render(spec, idx, level, out)?;
    }
    Ok(())
}

pub fn render_string(spec: &Specification) -> String {
    let mut out = Vec::new();
    // INVARIANT: Writing to a vector cannot fail, and everything written is UTF-8.
    render(spec, &mut out).unwrap();
    String::from_utf8(out).unwrap()
}

/// Text that is safe as the content of an element and as a quoted attribute
pub(crate) fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
