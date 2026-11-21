use crate::{
    node::markup::{IdxMarker, Item, Markup},
    syntax::{
        location::Span,
        parser::{Parser, path, reference},
        tokenizer::markup::{Kind, Token, tokenize},
    },
};

pub type TokenArena = super::TokenArena<Token, IdxMarker>;
pub type ParserArena = super::ParserArena<Token, Markup, IdxMarker>;

impl Parser<'_> {
    pub(super) fn markup(&mut self, span: Span) -> crate::node::markup::Idx {
        let mark = self.mark();
        let input = &self.source[span.range()];
        let tokens = tokenize(input);

        let markup = markup(input, &tokens, &mut self.references);
        self.markups.tokens.extend(tokens.into_iter());

        self.relocate(mark, span.start);
        self.markups.push_from(markup, mark.markup)
    }
}

fn markup(text: &str, tokens: &Vec<Token>, refs: &mut reference::ParserArena) -> Markup {
    let mut items: Vec<Item> = Vec::new();
    for tok in tokens {
        let item = match tok.kind {
            Kind::Text | Kind::UnfinishedReference => Item::Text(tok.text(text).to_string()),
            Kind::Indent => Item::Indent(tok.text(text).to_string()),
            Kind::Newline => Item::Newline,
            Kind::Reference => Item::Reference(reference(tok, text, refs)),
            Kind::ReferenceStart | Kind::ReferenceEnd => continue,
        };
        match (items.last_mut(), item) {
            (Some(Item::Text(last)), Item::Text(text)) => last.push_str(&text),
            (_, item) => items.push(item),
        }
    }
    Markup::from(items)
}

fn reference(
    token: &Token,
    text: &str,
    refs: &mut reference::ParserArena,
) -> crate::node::reference::Idx {
    let (path, tokens) = path::parse(token.text(text), token.span);
    let start = refs.tokens.len();
    refs.tokens.extend(tokens.into_iter());
    let range = start..refs.tokens.len();
    refs.push(path, range)
}

pub fn parse(text: &str, refs: &mut reference::ParserArena) -> Markup {
    let tokens = tokenize(text);
    markup(text, &tokens, refs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node::{path::Path, reference as node_reference};

    fn parse_alone(text: &str) -> (Markup, node_reference::Arena<Path>, Vec<Span>) {
        let (markup, paths, spans, _) = parse_with_tokens(text);
        (markup, paths, spans)
    }

    fn parse_with_tokens(
        text: &str,
    ) -> (Markup, node_reference::Arena<Path>, Vec<Span>, Vec<Token>) {
        let mut references = reference::ParserArena::default();
        let markup = parse(text, &mut references);
        let (paths, ref_tokens) = references.finish();
        (markup, paths, ref_tokens.spans(), tokenize(text))
    }

    #[test]
    fn keeps_every_token() {
        let text = "see [[a]] and [[b";
        let (_, _, _, tokens) = parse_with_tokens(text);
        let rebuilt: String = tokens.iter().map(|t| t.text(text)).collect();
        assert_eq!(rebuilt, text);
    }

    #[test]
    fn what_is_not_a_reference_is_text() {
        let text = "see [[a]] and [[b and [[]] and [[c d]] and {e} \\frac{f}{g}";
        let (markup, paths, _) = parse_alone(text);

        assert_eq!(paths.len(), 1);
        assert_eq!(markup.items.len(), 3);
        assert_eq!(
            render(markup.items.into_iter(), &paths),
            "see [[a]] and [[b and [[]] and [[c d]] and {e} \\frac{f}{g}"
        );
    }

    #[test]
    fn references_next_to_each_other_are_two_references() {
        let (markup, paths, spans) = parse_alone("[[a]][[b]]");
        let found: Vec<_> = markup.references().map(|r| paths[r].as_str()).collect();
        assert_eq!(found, ["a", "b"]);
        assert_eq!(spans.len(), 2);
    }

    fn render(items: impl Iterator<Item = Item>, paths: &node_reference::Arena<Path>) -> String {
        items
            .map(|item| match item {
                Item::Text(text) | Item::Indent(text) => text,
                Item::Newline => "\n".to_string(),
                Item::Reference(r) => format!("[[{}]]", paths[r].as_str()),
            })
            .collect()
    }

    #[test]
    fn items_are_the_text_as_it_is_written() {
        let text = "\n  a [[r]]\n\t b\n";
        let (markup, paths, _) = parse_alone(text);
        assert_eq!(render(markup.items.into_iter(), &paths), text);
    }

    #[test]
    fn unindents_like_the_unindent_crate() {
        for text in [
            "",
            "a",
            "\n  a\n  b",
            "a\n    b\n  c",
            "  a\n\n  b\n",
            "x\n\t a\n\t b\n   ",
            "é\n  é [[r]]\n  z",
            "\n    some [[ref]]\n      more [[other]]\n  ",
            "first\n  \n    third\n",
            "\n  \n    a\n  [[r]] b\n",
        ] {
            let (markup, paths, _) = parse_alone(text);
            assert_eq!(
                render(markup.unindented(), &paths),
                unindent::unindent(text),
                "{text:?}"
            );
        }
    }

    #[test]
    fn references_are_relative_to_the_text() {
        let text = "\n    some [[first]]\n      more [[second]]\n  ";
        let (_, _, spans) = parse_alone(text);

        let found: Vec<_> = spans.iter().map(|s| &text[s.range()]).collect();
        assert_eq!(found, ["first", "second"]);
        // The first line of the text is empty, then the references are on lines 1 and 2
        assert_eq!((spans[0].start.line, spans[0].start.column), (1, 11));
        assert_eq!((spans[1].start.line, spans[1].start.column), (2, 13));
    }
}
