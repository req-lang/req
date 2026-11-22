use reqlang::{
    node::{expression::Idx, tree::Tree},
    renderer::formatter::expression::{self, Ctx},
};

use crate::html::escape;

/// A formal expression is code, written as the formatter writes it. The page highlights `language-req`.
pub fn render<W: std::io::Write>(tree: &Tree, root: Idx, out: &mut W) -> std::io::Result<()> {
    let mut code = Vec::new();
    // At level 1 the expression is laid out on several lines, indented by one level
    expression::format(tree, root, Ctx::from(1), &mut code)?;
    // INVARIANT: The formatter only writes UTF-8.
    let code = String::from_utf8(code).unwrap();

    let code: Vec<&str> = code
        .lines()
        .map(|line| line.strip_prefix("  ").unwrap_or(line))
        .collect();
    writeln!(
        out,
        "<pre><code class=\"language-req\">{}</code></pre>",
        escape(&code.join("\n"))
    )
}
