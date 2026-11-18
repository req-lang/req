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

        let errs = errors(&tokens).into_iter().map(|err| err + span.start);
        let markup = markup(input, &tokens, &mut self.references);
        self.markups.tokens.extend(tokens.into_iter());

        self.relocate(mark, span.start);
        self.errors.extend(errs);
        self.markups.push_from(markup, mark.markup)
    }
}

fn errors(tokens: &Vec<Token>) -> Vec<Span> {
    let mut errs = Vec::new();

    for (idx, tok) in tokens.iter().enumerate() {
        let end = tokens.get(idx + 2);
        if tok.kind == Kind::ReferenceStart && end.map(|i| i.kind) != Some(Kind::ReferenceEnd) {
            errs.push(tok.span);
        }
    }

    errs
}

fn markup(text: &str, tokens: &Vec<Token>, refs: &mut reference::ParserArena) -> Markup {
    let items: Vec<_> = tokens
        .iter()
        .filter_map(|tok| match tok.kind {
            Kind::Text => Some(Item::Text(tok.text(text).to_string())),
            Kind::Indent => Some(Item::Indent(tok.text(text).to_string())),
            Kind::Newline => Some(Item::Newline),
            Kind::Reference => Some(Item::Reference(reference(tok, text, refs))),
            _ => None,
        })
        .collect();
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

pub fn parse(text: &str, refs: &mut reference::ParserArena) -> (Markup, Vec<Span>) {
    let tokens = tokenize(text);

    let errs = errors(&tokens).into_iter().map(|err| err).collect();
    let markup = markup(text, &tokens, refs);
    (markup, errs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node::{path::Path, reference as node_reference};

    fn parse_alone(text: &str) -> (Markup, node_reference::Arena<Path>, Vec<Span>) {
        let (markup, paths, spans, _, _) = parse_with_tokens(text);
        (markup, paths, spans)
    }

    fn parse_with_tokens(
        text: &str,
    ) -> (
        Markup,
        node_reference::Arena<Path>,
        Vec<Span>,
        Vec<Token>,
        Vec<Span>,
    ) {
        let mut references = reference::ParserArena::default();
        let (markup, errors) = parse(text, &mut references);
        let (paths, ref_tokens) = references.finish();
        (markup, paths, ref_tokens.spans(), tokenize(text), errors)
    }

    #[test]
    fn keeps_every_token() {
        let text = "see {a} and {b";
        let (_, _, _, tokens, _) = parse_with_tokens(text);
        let rebuilt: String = tokens.iter().map(|t| t.text(text)).collect();
        assert_eq!(rebuilt, text);
    }

    #[test]
    fn unclosed_reference_is_a_reference_and_an_error() {
        let text = "see {a} and {b";
        let (markup, paths, _, _, errors) = parse_with_tokens(text);

        // The reference is kept, the missing brace is reported at the `{`
        assert_eq!(paths.len(), 2);
        assert_eq!(render(markup.items.into_iter(), &paths), "see {a} and {b}");
        assert_eq!(errors.len(), 1);
        assert_eq!(&text[errors[0].range()], "{");
        assert_eq!(errors[0].start.column, 12);
    }

    #[test]
    fn empty_reference_is_a_reference() {
        let (markup, paths, spans) = parse_alone("see {}");
        assert_eq!(markup.references().count(), 1);
        assert_eq!(paths[markup.references().next().unwrap()].as_str(), "");
        assert_eq!(spans.len(), 1);
    }

    fn render(items: impl Iterator<Item = Item>, paths: &node_reference::Arena<Path>) -> String {
        items
            .map(|item| match item {
                Item::Text(text) | Item::Indent(text) => text,
                Item::Newline => "\n".to_string(),
                Item::Reference(r) => format!("{{{}}}", paths[r].as_str()),
            })
            .collect()
    }

    #[test]
    fn items_are_the_text_as_it_is_written() {
        let text = "\n  a {r}\n\t b\n";
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
            "é\n  é {r}\n  z",
            "\n    some {ref}\n      more {other}\n  ",
            "first\n  \n    third\n",
            "\n  \n    a\n  {r} b\n",
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
        let text = "\n    some {first}\n      more {second}\n  ";
        let (_, _, spans) = parse_alone(text);

        let found: Vec<_> = spans.iter().map(|s| &text[s.range()]).collect();
        assert_eq!(found, ["first", "second"]);
        // The first line of the text is empty, then the references are on lines 1 and 2
        assert_eq!((spans[0].start.line, spans[0].start.column), (1, 10));
        assert_eq!((spans[1].start.line, spans[1].start.column), (2, 12));
    }
}
