use std::ops::Range;

use crate::{
    node::{arena, tree::Tree},
    syntax::{
        location::{Position, Span, SpanLike, hull},
        tokenizer::entity::tokenize,
    },
    utils::{children::ChildrenIterMut, cursor::Cursor},
};

pub mod entity;
pub mod expression;
pub mod markup;
pub mod path;

#[derive(Debug, Clone)]
pub struct TokenArena<T, M> {
    tokens: Vec<T>,
    ranges: arena::Arena<Range<usize>, M>,
}

impl<T, M> Default for TokenArena<T, M> {
    fn default() -> Self {
        Self {
            tokens: Default::default(),
            ranges: Default::default(),
        }
    }
}

impl<T, M> TokenArena<T, M> {
    pub fn ranges(&self) -> &arena::Arena<Range<usize>, M> {
        &self.ranges
    }

    pub fn tokens(&self) -> &[T] {
        &self.tokens
    }

    pub fn tokens_of(&self, idx: arena::idx::Idx<M>) -> &[T] {
        &self.tokens[self.ranges[idx].clone()]
    }

    pub fn span_of(&self, idx: arena::idx::Idx<M>) -> Span
    where
        T: SpanLike,
    {
        hull(self.tokens_of(idx).iter().map(|t| t.span()))
    }

    pub fn spans(&self) -> Vec<Span>
    where
        T: SpanLike,
    {
        self.ranges.indices().map(|idx| self.span_of(idx)).collect()
    }
}

pub struct ParserArena<T, N, M> {
    nodes: arena::Arena<N, M>,
    ranges: arena::Arena<Range<usize>, M>,
    tokens: Cursor<T>,
}

impl<T, N, M> Default for ParserArena<T, N, M> {
    fn default() -> Self {
        Self {
            nodes: Default::default(),
            ranges: Default::default(),
            tokens: Default::default(),
        }
    }
}

impl<T, N, M, U: Iterator<Item = T>> From<U> for ParserArena<T, N, M> {
    fn from(value: U) -> Self {
        Self {
            nodes: Default::default(),
            ranges: Default::default(),
            tokens: Cursor::new(value.collect()),
        }
    }
}

impl<T, N, M> ParserArena<T, N, M> {
    pub fn push(&mut self, node: N, tokens: Range<usize>) -> arena::idx::Idx<M> {
        self.nodes.push(node);
        self.ranges.push(tokens);
        self.nodes.last_idx()
    }

    pub fn push_from(&mut self, node: N, start: usize) -> arena::idx::Idx<M> {
        self.push(node, start..self.tokens.position())
    }

    pub fn range_of(&self, idx: arena::idx::Idx<M>) -> Range<usize> {
        self.ranges[idx].clone()
    }

    pub fn extend(&mut self, other: Self) -> Option<arena::idx::Idx<M>>
    where
        N: ChildrenIterMut<arena::idx::Idx<M>>,
    {
        let nodes = self.nodes.len();
        let tokens = self.tokens.len();
        let last = other.nodes.len().checked_sub(1)?;

        for (mut node, range) in other.nodes.into_iter().zip(other.ranges) {
            for child in node.children_iter_mut() {
                *child = (usize::from(*child) + nodes).into();
            }
            self.push(node, range.start + tokens..range.end + tokens);
        }
        for token in other.tokens.into_tokens() {
            self.tokens.push(token);
        }
        Some((last + nodes).into())
    }

    pub fn finish(self) -> (arena::Arena<N, M>, TokenArena<T, M>) {
        let tokens = TokenArena {
            tokens: self.tokens.into_tokens(),
            ranges: self.ranges,
        };
        (self.nodes, tokens)
    }
}

/// What the parser knows about where the nodes come from
#[derive(Default, Debug, Clone)]
pub struct Info {
    pub entities: entity::TokenArena,
    pub expression: expression::TokenArena,
    pub references: reference::TokenArena,
    pub identifiers: identifier::TokenArena,
    pub markups: markup::TokenArena,
    pub errors: Vec<Span>,
}

/// Parses `input` into its tree, and where each node comes from
pub fn parse(input: &str) -> (Tree, Info) {
    Parser::new().parse(input)
}

#[derive(Default)]
struct Parser<'i> {
    entities: entity::ParserArena,
    expressions: expression::ParserArena,
    references: reference::ParserArena,
    identifiers: identifier::ParserArena,
    markups: markup::ParserArena,
    errors: Vec<Span>,
    source: &'i str,
}

impl<'i> Parser<'i> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn parse(mut self, input: &'i str) -> (Tree, Info) {
        self.source = input;
        self.entities.tokens = Cursor::new(tokenize(input).collect());
        self.entity(None);

        let (entities, entity_tokens) = self.entities.finish();
        let (expressions, expression_tokens) = self.expressions.finish();
        let (references, reference_tokens) = self.references.finish();
        let (identifiers, identifier_tokens) = self.identifiers.finish();
        let (markups, markup_tokens) = self.markups.finish();
        let tree = Tree {
            entities,
            expressions,
            references,
            identifiers,
            markups,
        };
        let info = Info {
            entities: entity_tokens,
            expression: expression_tokens,
            references: reference_tokens,
            identifiers: identifier_tokens,
            markups: markup_tokens,
            errors: self.errors,
        };
        (tree, info)
    }

    /// Where the spans of a text parsed on its own start: see [Self::relocate]
    fn mark(&self) -> Mark {
        Mark {
            expression: self.expressions.tokens.len(),
            identifier: self.identifiers.tokens.len(),
            reference: self.references.tokens.len(),
            markup: self.markups.tokens.len(),
        }
    }

    /// Moves the tokens parsed since `mark` to where their text is in the file. It was parsed on
    /// its own, so their spans are relative to the start of the text, at `origin` in the file.
    /// The span of a syntax error is returned by the parser, whoever calls it moves it.
    fn relocate(&mut self, mark: Mark, origin: Position) {
        for token in self.expressions.tokens.since_mut(mark.expression) {
            token.span = token.span + origin;
        }
        for token in self.markups.tokens.since_mut(mark.markup) {
            token.span = token.span + origin;
        }
        let paths = [
            self.identifiers.tokens.since_mut(mark.identifier),
            self.references.tokens.since_mut(mark.reference),
        ];
        for token in paths.into_iter().flatten() {
            token.span = token.span + origin;
        }
    }
}

/// How far the tokens of the parser were at some point
#[derive(Clone, Copy)]
struct Mark {
    expression: usize,
    identifier: usize,
    reference: usize,
    markup: usize,
}

pub mod reference {
    use crate::{
        node::{
            path::Path,
            reference::{Idx, IdxMarker},
        },
        syntax::{location::Span, parser::path, tokenizer::path::Token},
    };

    pub type ParserArena = super::ParserArena<Token, Path, IdxMarker>;
    pub type TokenArena = super::TokenArena<Token, IdxMarker>;

    impl super::Parser<'_> {
        pub(super) fn reference(&mut self, span: Span) -> Idx {
            let (path, tokens) = path::parse(&self.source[span.range()], span);
            let start = self.references.tokens.len();
            for token in tokens {
                self.references.tokens.push(token);
            }
            let range = start..self.references.tokens.len();
            self.references.push(path, range)
        }
    }
}

pub mod identifier {
    use crate::{
        node::{identifier::IdxMarker, path::Path},
        syntax::tokenizer::path::Token,
    };

    pub type ParserArena = super::ParserArena<Token, Path, IdxMarker>;
    pub type TokenArena = super::TokenArena<Token, IdxMarker>;
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use crate::{
        mock::{self, generator::Generate},
        node::{
            entity::{self, Requirement, TraceabilityKind},
            tree::Tree,
        },
        renderer,
        syntax::parser::parse,
        utils::children::ChildrenMut,
    };

    /// Stores `node` after its `children`, the order in which the parser stores entities
    fn push(draft: &mut Tree, mut node: entity::Node, children: &[entity::Idx]) -> entity::Idx {
        if !children.is_empty() {
            node.variant
                .children_mut()
                .expect("node has no children")
                .extend(children);
        }
        let idx = draft.entities.len().into();
        draft.entities.push(node);
        idx
    }

    #[test]
    fn parses_simple_input() {
        let input = "package x package";
        let result = parse(input);

        let mut expected = Tree::default();
        push(&mut expected, mock::entity::package("x"), &[]);

        assert!(result.1.errors.is_empty());
        assert_eq!(result.0, expected);
    }

    #[test]
    fn takes_only_the_first_root_if_several() {
        let input = "package x package package y package";
        let result = parse(input);

        let mut expected = Tree::default();
        push(&mut expected, mock::entity::package("x"), &[]);

        assert!(result.1.errors.is_empty());
        assert_eq!(result.0, expected);
    }

    #[test]
    fn parses_input_with_nested_packages() {
        let input = "package x package p package package";
        let result = parse(input);
        assert!(result.1.errors.is_empty());

        let mut expected = Tree::default();
        let p = push(&mut expected, mock::entity::package("p"), &[]);
        push(&mut expected, mock::entity::package("x"), &[p]);

        assert_eq!(result.0, expected);
    }

    #[test]
    fn parses_input_with_nested_part_and_package() {
        let input = "package x part p part package";
        let result = parse(input);
        assert!(result.1.errors.is_empty());

        let mut expected = Tree::default();
        let p = push(&mut expected, mock::entity::part("p"), &[]);
        push(&mut expected, mock::entity::package("x"), &[p]);

        assert_eq!(result.0, expected);
    }

    #[test]
    fn parses_input_with_single_attribute() {
        let input = "package x part p let u in real part package";
        let result = parse(input);
        assert!(result.1.errors.is_empty());

        let mut expected = Tree::default();
        let u = mock::entity::attribute(&mut expected, "u", "real");
        let u = push(&mut expected, u, &[]);
        let p = push(&mut expected, mock::entity::part("p"), &[u]);
        push(&mut expected, mock::entity::package("x"), &[p]);

        assert_eq!(result.0, expected);
    }

    #[test]
    fn parses_input_with_part_and_attribute() {
        let input = "package x part p let u in real \\m\\ let v in integer part package";
        let result = parse(input);
        assert!(result.1.errors.is_empty());

        let mut expected = Tree::default();
        let mut u = mock::entity::attribute(&mut expected, "u", "real");
        let unit = "m".to_string();
        u.variant.attribute_mut().unwrap().unit = Some(unit);
        let u = push(&mut expected, u, &[]);

        let v = mock::entity::attribute(&mut expected, "v", "integer");
        let v = push(&mut expected, v, &[]);

        let p = push(&mut expected, mock::entity::part("p"), &[u, v]);
        push(&mut expected, mock::entity::package("x"), &[p]);

        assert_eq!(result.0, expected);
    }

    #[test]
    fn parses_enum_type() {
        let input = "package x type T has a | b | c type package";
        let result = parse(input);
        assert!(result.1.errors.is_empty());

        let mut expected = Tree::default();
        let t = push(&mut expected, mock::entity::typ("T", &["a", "b", "c"]), &[]);
        push(&mut expected, mock::entity::package("x"), &[t]);

        assert_eq!(result.0, expected);
    }

    #[test]
    fn keeps_one_span_per_node() {
        let input = "package x part p let u in real + 1 part package";
        let (tree, info) = parse(input);
        assert!(info.errors.is_empty());

        assert_eq!(info.entities.ranges.len(), tree.entities.len());
        assert_eq!(info.expression.ranges.len(), tree.expressions.len());
    }

    #[test]
    fn spans_follow_the_order_of_the_entities() {
        let input = "package x part p part part q part package";
        let (tree, info) = parse(input);
        assert!(info.errors.is_empty());

        // A parent is stored after its children, and so is its span
        let starts: Vec<_> = info
            .entities
            .spans()
            .iter()
            .map(|s| s.start.index)
            .collect();
        assert_eq!(tree.entities.len(), 3);
        assert_eq!(
            starts,
            vec![
                input.find("part p").unwrap(),
                input.find("part q").unwrap(),
                0
            ]
        );
    }

    /// Text of the root expression of the only requirement of `input`
    fn root_expression_text(input: &str) -> (&str, crate::syntax::location::Span) {
        let (tree, info) = parse(input);
        assert!(info.errors.is_empty());
        let root = tree.expressions.indices().last().unwrap();
        let span = info.expression.span_of(root);
        (&input[span.range()], span)
    }

    #[test]
    fn relocates_what_is_parsed_in_the_middle_of_the_file() {
        // Multi-line expression with identifiers, a markup with references, and then an
        // expression that fails
        let input = "package p\n  requirement r is\n    aa +\n    (bb * cc)\n  requirement\n  requirement q is\n    @@\n      see [[dd]]\n    @@\n  requirement\n  requirement s is\n    ee +\n  requirement\npackage";
        let (_, info) = parse(input);

        let text = |s: &crate::syntax::location::Span| &input[s.range()];
        let identifiers = info.identifiers.spans();
        // `ee` is read before the expression fails, and it stays
        assert_eq!(
            identifiers.iter().map(text).collect::<Vec<_>>(),
            ["aa", "bb", "cc", "ee"]
        );
        // `cc` is on the 4th line, after `(bb * `
        assert_eq!(
            (identifiers[2].start.line, identifiers[2].start.column),
            (3, 10)
        );
        let references = info.references.spans();
        assert_eq!(references.iter().map(text).collect::<Vec<_>>(), ["dd"]);
        assert_eq!(
            (references[0].start.line, references[0].start.column),
            (7, 12)
        );

        // The error is in the failing expression, on its line
        let error = info.errors[0];
        assert_eq!(info.errors.len(), 1);
        let failing = input.find("ee +").unwrap();
        assert!((failing..failing + "ee +".len() + 1).contains(&error.start.index));
        assert_eq!(error.start.line, 11);
    }

    #[test]
    fn the_expressions_of_a_file_share_the_arenas() {
        use crate::node::expression::Variant;

        let input = "package p\n  requirement r is\n    a + b\n  requirement\n  requirement s is\n    c * (d - e)\n  requirement\npackage";
        let (tree, info) = parse(input);
        assert!(info.errors.is_empty());

        let text = |idx| &input[info.expression.span_of(idx).range()];
        let mut operations = Vec::new();
        for (idx, node) in tree.expressions.enumerate() {
            if let Variant::BinOp(op) = &node.variant {
                // The operands are nodes of the same expression
                assert!(text(idx).contains(text(op.left)), "{}", text(idx));
                assert!(text(idx).contains(text(op.right)), "{}", text(idx));
                operations.push(text(idx));
            }
        }
        // The node inside parentheses takes their tokens
        assert_eq!(operations, ["a + b", "(d - e)", "c * (d - e)"]);
    }

    #[test]
    fn an_incomplete_entity_is_an_error() {
        let cases = [
            ("package A\n  part\npackage\n", vec!["part"]),
            ("package A\n  let in\npackage\n", vec!["let"]),
            (
                "package A\n  let x\n  part B\n  part\npackage\n",
                vec!["part"],
            ),
            ("package A\n  type T has\npackage\n", vec!["type"]),
            (
                "package A\n  requirement\n  is\n    true\n  requirement\npackage\n",
                vec!["requirement", "requirement"],
            ),
        ];
        for (input, expected) in cases {
            let (_, info) = parse(input);
            let flagged: Vec<_> = info.errors.iter().map(|s| &input[s.range()]).collect();
            assert_eq!(flagged, expected, "{input:?}");
        }
    }

    #[test]
    fn an_expression_with_an_error_has_no_nodes_but_keeps_its_tokens() {
        let input =
            "package p\n  let x in real\n  requirement r on x is\n    x >\n  requirement\npackage";
        let (tree, info) = parse(input);
        assert_eq!(info.errors.len(), 1);
        // The requirement is dropped with its expression
        assert_eq!(tree.entities.len(), 2);
        // `real` is the only expression
        assert_eq!(tree.expressions.len(), 1);

        let text = |t: &crate::syntax::tokenizer::expression::Token| &input[t.span.range()];
        let tokens: Vec<_> = info.expression.tokens.iter().map(text).collect();
        assert_eq!(tokens, ["real", "x", ">"]);
        // The tokens are where they are in the file
        let line = info.expression.tokens[2].span.start.line;
        assert_eq!(line, 3);
    }

    #[test]
    fn references_and_identifiers_are_made_of_names_and_separators() {
        use crate::syntax::tokenizer::path::Kind::{Segment, Separator};

        let input = "package p\n  requirement r refines A::b is\n    @@\n      see [[C::d::]] and [[]]\n    @@\n  requirement\n  requirement s is\n    x::y + z\n  requirement\npackage";
        let (tree, info) = parse(input);
        assert!(info.errors.len() <= 1);
        assert_eq!(tree.references.len(), 2);
        assert_eq!(tree.identifiers.len(), 2);

        fn tokens<'a, M>(
            arena: &crate::syntax::parser::TokenArena<crate::syntax::tokenizer::path::Token, M>,
            idx: usize,
            input: &'a str,
        ) -> Vec<(crate::syntax::tokenizer::path::Kind, &'a str, usize, usize)> {
            arena
                .tokens_of(idx.into())
                .iter()
                .map(|t| {
                    (
                        t.kind,
                        &input[t.span.range()],
                        t.span.start.line,
                        t.span.start.column,
                    )
                })
                .collect::<Vec<_>>()
        }
        assert_eq!(
            tokens(&info.references, 0, input),
            [
                (Segment, "A", 1, 24),
                (Separator, "::", 1, 25),
                (Segment, "b", 1, 27)
            ]
        );
        assert_eq!(
            tokens(&info.references, 1, input),
            [
                (Segment, "C", 3, 12),
                (Separator, "::", 3, 13),
                (Segment, "d", 3, 15),
                (Separator, "::", 3, 16),
            ]
        );

        assert_eq!(
            tokens(&info.identifiers, 0, input),
            [
                (Segment, "x", 7, 4),
                (Separator, "::", 7, 5),
                (Segment, "y", 7, 7)
            ]
        );
        assert_eq!(tokens(&info.identifiers, 1, input), [(Segment, "z", 7, 11)]);
    }

    #[test]
    fn locates_references_and_identifiers() {
        let input = "package p\n  requirement r refines a is\n    @@\n      see [[b]]\n    @@\n  requirement\n  requirement s is\n    xx + (yy)\n  requirement\npackage";
        let (tree, info) = parse(input);
        assert!(info.errors.is_empty());

        let text = |s: &crate::syntax::location::Span| &input[s.range()];
        let spans = info.references.spans();
        let references: Vec<_> = spans.iter().map(text).collect();
        assert_eq!(references, ["a", "b"]);
        assert_eq!(spans.len(), tree.references.len());
        assert_eq!((spans[1].start.line, spans[1].start.column), (3, 12));

        let spans = info.identifiers.spans();
        let identifiers: Vec<_> = spans.iter().map(text).collect();
        assert_eq!(identifiers, ["xx", "yy"]);
        assert_eq!(spans.len(), tree.identifiers.len());
    }

    #[test]
    fn expression_span_covers_the_whole_operation() {
        for expr in [
            "a + b",
            "- a",
            "a !",
            "f(a, b)",
            "(a + b) * c",
            "a + (b * c)",
        ] {
            let input = format!("package p requirement r is {expr} requirement package");
            assert_eq!(root_expression_text(&input).0, expr);
        }
    }

    #[test]
    fn expression_span_is_exact_on_following_lines() {
        let input = "package p\n  requirement r is\n    a +\n    (b * c)\n  requirement\npackage";
        let (text, span) = root_expression_text(input);
        assert_eq!(text, "a +\n    (b * c)");
        assert_eq!((span.start.line, span.start.column), (2, 4));
        assert_eq!((span.end.line, span.end.column), (3, 11));
    }

    #[test]
    fn parses_simple_generated_input() {
        let mut generator = mock::generator::Simple::new();
        generator.packages = 2;
        generator.depth = 1;
        generator.words = 2;
        let tree = generator.generate();
        assert_eq!(tree.entities.len(), generator.size() as usize);
    }

    #[test]
    fn parses_reference_text_generated_input() {
        let mut generator = mock::generator::ReferenceText::new();
        generator.requirements = 2;
        generator.lines = 3;
        generator.per_line = 2;
        generator.words = 0;
        generator.segments = 4;
        let text = renderer::formatter::format_string(&generator.generate(), 0);

        let (tree, info) = parse(&text);

        assert!(info.errors.is_empty());
        assert_eq!(tree.entities.len(), generator.size() as usize);
        assert_eq!(tree.references.len() as u64, generator.references());
        assert_eq!(tree.references[0.into()].as_str(), "s0_0::s0_1::s0_2::s0_3");
    }

    #[test]
    fn parses_requirement_with_refinement_traceability() {
        let (tree, req) =
            get_requirement("package p requirement x refines a is @@ text @@ requirement package");
        assert_eq!(req.traceability.len(), 1);
        assert_eq!(req.traceability[0].kind, TraceabilityKind::Refinement);
        assert_eq!(tree.references[req.traceability[0].target].as_str(), "a");
    }

    #[test]
    fn parses_requirement_with_multiple_traceability_links() {
        let (tree, req) = get_requirement(
            "package p requirement x refines a specializes b derives c is @@ text @@ requirement package",
        );
        assert_eq!(req.traceability.len(), 3);
        assert_eq!(req.traceability[0].kind, TraceabilityKind::Refinement);
        assert_eq!(tree.references[req.traceability[0].target].as_str(), "a");
        assert_eq!(req.traceability[1].kind, TraceabilityKind::Specialization);
        assert_eq!(tree.references[req.traceability[1].target].as_str(), "b");
        assert_eq!(req.traceability[2].kind, TraceabilityKind::Derivation);
        assert_eq!(tree.references[req.traceability[2].target].as_str(), "c");
    }

    #[test]
    fn parses_requirement_with_constraint() {
        let (tree, req) =
            get_requirement("package p requirement x on y is @@ text @@ requirement package");
        assert_eq!(
            req.restriction.map(|r| tree.references[r].as_str()),
            Some("y")
        );
        assert!(req.traceability.is_empty());
        assert!(req.allocation.is_none());
    }

    #[test]
    fn parses_requirement_with_allocation() {
        let (tree, req) =
            get_requirement("package p requirement x via z is @@ text @@ requirement package");
        assert_eq!(
            req.allocation.map(|r| tree.references[r].as_str()),
            Some("z")
        );
        assert!(req.traceability.is_empty());
        assert!(req.restriction.is_none());
    }

    #[test]
    fn parses_requirement_with_all_advanced_traceability() {
        let (tree, req) = get_requirement(
            "package p requirement x on y via z refines a specializes b is @@ text @@ requirement package",
        );
        assert_eq!(
            req.restriction.map(|r| tree.references[r].as_str()),
            Some("y")
        );
        assert_eq!(
            req.allocation.map(|r| tree.references[r].as_str()),
            Some("z")
        );
        assert_eq!(req.traceability.len(), 2);
        assert_eq!(req.traceability[0].kind, TraceabilityKind::Refinement);
        assert_eq!(tree.references[req.traceability[0].target].as_str(), "a");
        assert_eq!(req.traceability[1].kind, TraceabilityKind::Specialization);
        assert_eq!(tree.references[req.traceability[1].target].as_str(), "b");
    }

    fn get_requirement(input: &str) -> (Tree, Requirement) {
        let (tree, info) = parse(input);
        assert!(info.errors.is_empty());
        let requirement = tree
            .entities
            .iter()
            .find_map(|n| n.variant.requirement().cloned())
            .expect("no requirement found");
        (tree, requirement)
    }

    #[test]
    fn attaches_comments_to_the_following_nested_entity() {
        use crate::node::markup::{Item, Markup};

        let input = "@@ first comment @@ package x @@ second comment @@ package y package package";
        let result = parse(input);
        assert!(result.1.errors.is_empty());

        let comment = |tree: &mut Tree, text: &str| {
            let markup = Markup::from(vec![Item::Text(text.to_string())]);
            Some(mock::entity::markup(tree, markup))
        };

        // The markups are stored in the order they are written
        let mut expected = Tree::default();
        let first = comment(&mut expected, "first comment");
        let second = comment(&mut expected, "second comment");
        let mut y = mock::entity::package("y");
        y.meta.comment = second;
        let y = push(&mut expected, y, &[]);
        let mut x = mock::entity::package("x");
        x.meta.comment = first;
        push(&mut expected, x, &[y]);

        assert_eq!(result.0, expected);
    }

    #[test]
    fn attaches_tags_to_the_following_nested_entity() {
        let input = "# kind outer # package x # kind inner # package y package package";
        let result = parse(input);
        assert!(result.1.errors.is_empty());

        let tag = |value: &str| entity::Tag::new("kind".to_string(), Some(value.to_string()));

        let mut expected = Tree::default();
        let mut y = mock::entity::package("y");
        y.meta.tags.push(tag("inner"));
        let y = push(&mut expected, y, &[]);
        let mut x = mock::entity::package("x");
        x.meta.tags.push(tag("outer"));
        push(&mut expected, x, &[y]);

        assert_eq!(result.0, expected);
    }
}
