use crate::{
    node::{
        self,
        tree::{Draft, Tree},
    },
    syntax::{
        location::Span,
        tokenizer::entity::{IntoLexeme, IntoPosition, IntoTokens},
    },
    utils::cursor::Cursor,
};

pub mod entity;
pub mod expression;
pub mod markup;

/// Where each node comes from in the source, indexed like the arenas of the [Tree]
#[derive(Default, Debug, Clone)]
pub struct Spans {
    pub entities: node::entity::Arena<Span>,
    pub expressions: node::expression::Arena<Span>,
}

/// What [Spans] is made of while parsing, in the same order as the [Draft]
#[derive(Default)]
struct SpansDraft {
    entities: Vec<Span>,
    expressions: Vec<Span>,
}

pub fn parse(input: &str) -> (Tree, Spans, Vec<Span>) {
    let mut tree = Draft::default();
    let mut spans = SpansDraft::default();
    let mut draft = entity::Draft::new(&mut tree, &mut spans);
    let mut errors = Vec::new();
    let tokens: Vec<_> = input
        .char_indices()
        .positioned()
        .lexemes()
        .tokens()
        .collect();
    let mut cursor = Cursor::new(&tokens);
    entity::parse_tree(&mut cursor, &mut draft, &mut errors);
    let spans = Spans {
        entities: spans.entities.into(),
        expressions: spans.expressions.into(),
    };
    (tree.into(), spans, errors)
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use crate::{
        mock::{self, generator::Generate},
        node::{
            entity::{self, Requirement, TraceabilityKind},
            tree::{Draft, Tree},
        },
        syntax::parser::parse,
        utils::children::ChildrenMut,
    };

    /// Stores `node` after its `children`, the order in which the parser stores entities
    fn push(draft: &mut Draft, mut node: entity::Node, children: &[entity::Idx]) -> entity::Idx {
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

        let mut expected = Draft::default();
        push(&mut expected, mock::entity::package("x"), &[]);

        assert!(result.2.is_empty());
        assert_eq!(result.0, Tree::from(expected));
    }

    #[test]
    fn takes_only_the_first_root_if_several() {
        let input = "package x package package y package";
        let result = parse(input);

        let mut expected = Draft::default();
        push(&mut expected, mock::entity::package("x"), &[]);

        assert!(result.2.is_empty());
        assert_eq!(result.0, Tree::from(expected));
    }

    #[test]
    fn parses_input_with_nested_packages() {
        let input = "package x package p package package";
        let result = parse(input);
        assert!(result.2.is_empty());

        let mut expected = Draft::default();
        let p = push(&mut expected, mock::entity::package("p"), &[]);
        push(&mut expected, mock::entity::package("x"), &[p]);

        assert_eq!(result.0, Tree::from(expected));
    }

    #[test]
    fn parses_input_with_nested_part_and_package() {
        let input = "package x part p part package";
        let result = parse(input);
        assert!(result.2.is_empty());

        let mut expected = Draft::default();
        let p = push(&mut expected, mock::entity::part("p"), &[]);
        push(&mut expected, mock::entity::package("x"), &[p]);

        assert_eq!(result.0, Tree::from(expected));
    }

    #[test]
    fn parses_input_with_single_attribute() {
        let input = "package x part p let u in real part package";
        let result = parse(input);
        assert!(result.2.is_empty());

        let mut expected = Draft::default();
        let u = mock::entity::attribute(&mut expected, "u", "real");
        let u = push(&mut expected, u, &[]);
        let p = push(&mut expected, mock::entity::part("p"), &[u]);
        push(&mut expected, mock::entity::package("x"), &[p]);

        assert_eq!(result.0, Tree::from(expected));
    }

    #[test]
    fn parses_input_with_part_and_attribute() {
        let input = "package x part p let u in real \\m\\ let v in integer part package";
        let result = parse(input);
        assert!(result.2.is_empty());

        let mut expected = Draft::default();
        let mut u = mock::entity::attribute(&mut expected, "u", "real");
        let unit = "m".to_string();
        u.variant.attribute_mut().unwrap().unit = Some(unit);
        let u = push(&mut expected, u, &[]);

        let v = mock::entity::attribute(&mut expected, "v", "integer");
        let v = push(&mut expected, v, &[]);

        let p = push(&mut expected, mock::entity::part("p"), &[u, v]);
        push(&mut expected, mock::entity::package("x"), &[p]);

        assert_eq!(result.0, Tree::from(expected));
    }

    #[test]
    fn parses_enum_type() {
        let input = "package x type T has a | b | c type package";
        let result = parse(input);
        assert!(result.2.is_empty());

        let mut expected = Draft::default();
        let t = push(&mut expected, mock::entity::typ("T", &["a", "b", "c"]), &[]);
        push(&mut expected, mock::entity::package("x"), &[t]);

        assert_eq!(result.0, Tree::from(expected));
    }

    #[test]
    fn keeps_one_span_per_node() {
        let input = "package x part p let u in real + 1 part package";
        let (tree, spans, errors) = parse(input);
        assert!(errors.is_empty());

        assert_eq!(spans.entities.len(), tree.entities.len());
        assert_eq!(spans.expressions.len(), tree.expressions.len());
    }

    #[test]
    fn spans_follow_the_order_of_the_entities() {
        let input = "package x part p part part q part package";
        let (tree, spans, errors) = parse(input);
        assert!(errors.is_empty());

        // A parent is stored after its children, and so is its span
        let starts: Vec<_> = spans.entities.iter().map(|s| s.start.index).collect();
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
        let (tree, _, errors) = parse(input);
        assert!(errors.is_empty());
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
        assert!(result.2.is_empty());

        let comment = |text: &str| Some(Markup::from(vec![Item::Text(text.to_string())]));

        let mut expected = Draft::default();
        let mut y = mock::entity::package("y");
        y.meta.comment = comment("second comment");
        let y = push(&mut expected, y, &[]);
        let mut x = mock::entity::package("x");
        x.meta.comment = comment("first comment");
        push(&mut expected, x, &[y]);

        assert_eq!(result.0, Tree::from(expected));
    }

    #[test]
    fn attaches_tags_to_the_following_nested_entity() {
        let input = "# kind outer # package x # kind inner # package y package package";
        let result = parse(input);
        assert!(result.2.is_empty());

        let tag = |value: &str| entity::Tag::new("kind".to_string(), Some(value.to_string()));

        let mut expected = Draft::default();
        let mut y = mock::entity::package("y");
        y.meta.tags.push(tag("inner"));
        let y = push(&mut expected, y, &[]);
        let mut x = mock::entity::package("x");
        x.meta.tags.push(tag("outer"));
        push(&mut expected, x, &[y]);

        assert_eq!(result.0, Tree::from(expected));
    }
}
