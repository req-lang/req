use itertools::Itertools;

use crate::syntax::{
    location::Span,
    parser::entity::Node,
    tokenizer::entity::{IntoLexeme, IntoPosition, IntoTokens},
};

pub mod entity;
pub mod expression;
pub mod markup;

pub fn parse(input: &str) -> Result<Vec<Node>, Span> {
    let mut arena = Vec::new();
    let mut tokens = input
        .char_indices()
        .positioned()
        .lexemes()
        .tokens()
        .multipeek();
    entity::parse(&mut tokens, &mut arena)?;
    Ok(arena)
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use crate::{
        mock::{self, generator::Generate},
        node::{
            self, NodeRef,
            entity::{Requirement, TraceabilityKind, Variant},
        },
        reference::Reference,
        renderer::formatter::format_string,
        syntax::{
            self, entity,
            parser::{expression, markup, parse},
        },
        utils::children::{ChildrenIter, IntoChildren},
    };

    #[test]
    fn parses_simple_input() {
        let input = "package x package";
        let result = parse(input);

        let expected = mock::entity::package("x");

        assert!(result.is_ok());

        let result = node::map(result.ok().unwrap(), entity::Node::from);
        assert_eq!(*result.borrow(), *expected.borrow());
    }

    #[test]
    fn parses_input_with_nested_packages() {
        let input = "package x package p package package";
        let result = parse(input);
        assert!(result.is_ok());

        let actual = result.ok().unwrap();
        let actual = node::map(actual, syntax::entity::Node::from);

        let expected = mock::entity::package("x");

        let first = mock::entity::package("p");

        {
            let mut expected_ref = expected.borrow_mut();
            let children = expected_ref.variant.children_mut().unwrap();
            children.push(first);
        }

        assert_eq!(*actual.borrow(), *expected.borrow());
    }

    #[test]
    fn parses_input_with_nested_part_and_package() {
        let input = "package x part p part package";
        let result = parse(input);
        assert!(result.is_ok());

        let actual = result.ok().unwrap();
        let actual = node::map(actual, syntax::entity::Node::from);

        let expected = mock::entity::package("x");

        let first = mock::entity::part("p");

        {
            let mut expected_ref = expected.borrow_mut();
            let children = expected_ref.variant.children_mut().unwrap();
            children.push(first);
        }

        assert_eq!(*actual.borrow(), *expected.borrow());
    }

    #[test]
    fn parses_input_with_single_attribute() {
        let input = "package x part p let u in real part package";
        let result = parse(input);
        assert!(result.is_ok());

        let actual = result.ok().unwrap();
        let actual = node::map(actual, syntax::entity::Node::from);

        let expected = mock::entity::package("x");

        let first = mock::entity::part("p");

        let second = mock::entity::attribute("u", "real");

        {
            let mut first_ref = first.borrow_mut();
            let children = first_ref.variant.children_mut().unwrap();
            children.push(second);
        }

        {
            let mut expected_ref = expected.borrow_mut();
            let children = expected_ref.variant.children_mut().unwrap();
            children.push(first);
        }

        assert_eq!(*actual.borrow(), *expected.borrow());
    }

    #[test]
    fn parses_input_with_part_and_attribute() {
        let input = "package x part p let u in real [m] let v in integer part package";
        let result = parse(input);
        assert!(result.is_ok());

        let actual = result.ok().unwrap();
        let actual = node::map(actual, syntax::entity::Node::from);

        let expected = mock::entity::package("x");

        let first = mock::entity::part("p");

        let second = mock::entity::attribute("u", "real");
        let unit = mock::expression::identifier("m");
        second.borrow_mut().variant.attribute_mut().unwrap().unit = Some(unit);

        let third = mock::entity::attribute("v", "integer");

        {
            let mut first_ref = first.borrow_mut();
            let children = first_ref.variant.children_mut().unwrap();
            children.push(second);
            children.push(third);
        }

        {
            let mut expected_ref = expected.borrow_mut();
            let children = expected_ref.variant.children_mut().unwrap();
            children.push(first);
        }

        assert_eq!(*actual.borrow(), *expected.borrow());
    }

    #[test]
    fn parses_simple_generated_input() {
        let mut generator = mock::generator::Simple::new();
        generator.packages = 2;
        generator.depth = 1;
        generator.words = 2;
        let expected = generator.generate();

        let input = format_string(expected.borrow(), 0);
        let result = parse(&input);
        assert!(result.is_ok());

        let actual = result.ok().unwrap();
        let actual = node::map(actual, syntax::entity::Node::from);

        assert_eq!(count_nodes(&*actual.borrow()), generator.size() as usize);
        assert_eq!(*actual.borrow(), *expected.borrow());
    }

    #[test]
    fn parses_requirement_with_refinement_traceability() {
        let req =
            get_requirement("package p requirement x refines a is @@ text @@ requirement package");
        assert_eq!(req.traceability.len(), 1);
        assert_eq!(req.traceability[0].kind, TraceabilityKind::Refinement);
        assert_eq!(req.traceability[0].target.value, "a");
    }

    #[test]
    fn parses_requirement_with_multiple_traceability_links() {
        let req = get_requirement(
            "package p requirement x refines a specializes b derives c is @@ text @@ requirement package",
        );
        assert_eq!(req.traceability.len(), 3);
        assert_eq!(req.traceability[0].kind, TraceabilityKind::Refinement);
        assert_eq!(req.traceability[0].target.value, "a");
        assert_eq!(req.traceability[1].kind, TraceabilityKind::Specialization);
        assert_eq!(req.traceability[1].target.value, "b");
        assert_eq!(req.traceability[2].kind, TraceabilityKind::Derivation);
        assert_eq!(req.traceability[2].target.value, "c");
    }

    #[test]
    fn parses_requirement_with_constraint() {
        let req = get_requirement("package p requirement x on y is @@ text @@ requirement package");
        assert_eq!(
            req.restriction.as_ref().map(|r| r.value.as_str()),
            Some("y")
        );
        assert!(req.traceability.is_empty());
        assert!(req.allocation.is_none());
    }

    #[test]
    fn parses_requirement_with_allocation() {
        let req =
            get_requirement("package p requirement x via z is @@ text @@ requirement package");
        assert_eq!(req.allocation.as_ref().map(|r| r.value.as_str()), Some("z"));
        assert!(req.traceability.is_empty());
        assert!(req.restriction.is_none());
    }

    #[test]
    fn parses_requirement_with_all_advanced_traceability() {
        let req = get_requirement(
            "package p requirement x on y via z refines a specializes b is @@ text @@ requirement package",
        );
        assert_eq!(
            req.restriction.as_ref().map(|r| r.value.as_str()),
            Some("y")
        );
        assert_eq!(req.allocation.as_ref().map(|r| r.value.as_str()), Some("z"));
        assert_eq!(req.traceability.len(), 2);
        assert_eq!(req.traceability[0].kind, TraceabilityKind::Refinement);
        assert_eq!(req.traceability[0].target.value, "a");
        assert_eq!(req.traceability[1].kind, TraceabilityKind::Specialization);
        assert_eq!(req.traceability[1].target.value, "b");
    }

    fn count_nodes(node: &syntax::entity::Node) -> usize {
        1 + node
            .variant
            .children_iter()
            .map(|c| count_nodes(&*c.borrow()))
            .sum::<usize>()
    }

    fn get_requirement(
        input: &str,
    ) -> Requirement<Reference, NodeRef<expression::Node>, markup::Node> {
        let actual = parse(input);
        assert!(actual.is_ok());

        let node = node::unwrap(actual.unwrap());
        let children = node.variant.into_children().unwrap();

        let entity = children
            .into_iter()
            .find(|e| matches!(e.borrow().variant, Variant::Requirement(_)))
            .expect("no requirement found");

        node::unwrap(entity).variant.into_requirement().unwrap()
    }
}
