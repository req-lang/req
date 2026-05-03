use itertools::Itertools;

use crate::{
    node::entity::Arena,
    syntax::{
        location::Span,
        parser::entity::Node,
        tokenizer::entity::{IntoLexeme, IntoPosition, IntoTokens},
    },
};

pub mod entity;
pub mod expression;
pub mod markup;

pub fn parse(input: &str) -> Result<Arena<Node>, Span> {
    let mut arena = Vec::new();
    let mut tokens = input
        .char_indices()
        .positioned()
        .lexemes()
        .tokens()
        .multipeek();
    entity::parse(&mut tokens, &mut arena)?;
    Ok(arena.into())
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use crate::{
        mock::{self, generator::Generate},
        node::{
            self,
            entity::{Arena, Requirement, TraceabilityKind},
        },
        syntax::{self, parser::parse},
        utils::children::ChildrenMut,
    };

    #[test]
    fn parses_simple_input() {
        let input = "package x package";
        let result = parse(input);

        let expected: Arena<_> = vec![mock::entity::package("x")].into();

        assert!(result.is_ok());

        let nodes = result.ok().unwrap();
        let ast: Arena<_> = nodes.into_iter().map(syntax::entity::Node::from).collect();
        assert_eq!(ast, expected);
    }

    #[test]
    fn parses_input_with_nested_packages() {
        let input = "package x package p package package";
        let result = parse(input);
        assert!(result.is_ok());

        let nodes = result.ok().unwrap();
        let ast: Arena<_> = nodes.into_iter().map(syntax::entity::Node::from).collect();

        let mut x = mock::entity::package("x");
        x.variant.children_mut().unwrap().push(1.into());
        let p = mock::entity::package("p");

        assert_eq!(ast, vec![x, p].into());
    }

    #[test]
    fn parses_input_with_nested_part_and_package() {
        let input = "package x part p part package";
        let result = parse(input);
        assert!(result.is_ok());

        let nodes = result.ok().unwrap();
        let ast: Arena<_> = nodes.into_iter().map(syntax::entity::Node::from).collect();

        let mut expected = mock::entity::package("x");
        expected.variant.children_mut().unwrap().push(1.into());
        let first = mock::entity::part("p");

        assert_eq!(ast, vec![expected, first].into());
    }

    #[test]
    fn parses_input_with_single_attribute() {
        let input = "package x part p let u in real part package";
        let result = parse(input);
        assert!(result.is_ok());

        let nodes = result.ok().unwrap();
        let ast: Arena<_> = nodes.into_iter().map(syntax::entity::Node::from).collect();

        let mut x = mock::entity::package("x");
        let mut p = mock::entity::part("p");
        let u = mock::entity::attribute("u", "real");

        x.variant.children_mut().unwrap().push(1.into());
        p.variant.children_mut().unwrap().push(2.into());

        assert_eq!(ast, vec![x, p, u].into());
    }

    #[test]
    fn parses_input_with_part_and_attribute() {
        let input = "package x part p let u in real [m] let v in integer part package";
        let result = parse(input);
        assert!(result.is_ok());

        let nodes = result.ok().unwrap();
        let ast: Arena<_> = nodes.into_iter().map(syntax::entity::Node::from).collect();

        let mut x = mock::entity::package("x");
        x.variant.children_mut().unwrap().push(1.into());

        let mut p = mock::entity::part("p");
        p.variant.children_mut().unwrap().push(2.into());
        p.variant.children_mut().unwrap().push(3.into());

        let mut u = mock::entity::attribute("u", "real");
        let unit = "m".to_string();
        u.variant.attribute_mut().unwrap().unit = Some(unit);

        let v = mock::entity::attribute("v", "integer");

        assert_eq!(ast, vec![x, p, u, v].into());
    }

    #[test]
    fn parses_simple_generated_input() {
        let mut generator = mock::generator::Simple::new();
        generator.packages = 2;
        generator.depth = 1;
        generator.words = 2;
        let nodes = generator.generate();
        assert_eq!(Vec::from(nodes).len(), generator.size() as usize);
    }

    #[test]
    fn parses_requirement_with_refinement_traceability() {
        let req =
            get_requirement("package p requirement x refines a is @@ text @@ requirement package");
        assert_eq!(req.traceability.len(), 1);
        assert_eq!(req.traceability[0].kind, TraceabilityKind::Refinement);
        assert_eq!(req.traceability[0].target.as_str(), "a");
    }

    #[test]
    fn parses_requirement_with_multiple_traceability_links() {
        let req = get_requirement(
            "package p requirement x refines a specializes b derives c is @@ text @@ requirement package",
        );
        assert_eq!(req.traceability.len(), 3);
        assert_eq!(req.traceability[0].kind, TraceabilityKind::Refinement);
        assert_eq!(req.traceability[0].target.as_str(), "a");
        assert_eq!(req.traceability[1].kind, TraceabilityKind::Specialization);
        assert_eq!(req.traceability[1].target.as_str(), "b");
        assert_eq!(req.traceability[2].kind, TraceabilityKind::Derivation);
        assert_eq!(req.traceability[2].target.as_str(), "c");
    }

    #[test]
    fn parses_requirement_with_constraint() {
        let req = get_requirement("package p requirement x on y is @@ text @@ requirement package");
        assert_eq!(req.restriction.as_ref().map(|r| r.as_str()), Some("y"));
        assert!(req.traceability.is_empty());
        assert!(req.allocation.is_none());
    }

    #[test]
    fn parses_requirement_with_allocation() {
        let req =
            get_requirement("package p requirement x via z is @@ text @@ requirement package");
        assert_eq!(req.allocation.as_ref().map(|r| r.as_str()), Some("z"));
        assert!(req.traceability.is_empty());
        assert!(req.restriction.is_none());
    }

    #[test]
    fn parses_requirement_with_all_advanced_traceability() {
        let req = get_requirement(
            "package p requirement x on y via z refines a specializes b is @@ text @@ requirement package",
        );
        assert_eq!(req.restriction.as_ref().map(|r| r.as_str()), Some("y"));
        assert_eq!(req.allocation.as_ref().map(|r| r.as_str()), Some("z"));
        assert_eq!(req.traceability.len(), 2);
        assert_eq!(req.traceability[0].kind, TraceabilityKind::Refinement);
        assert_eq!(req.traceability[0].target.as_str(), "a");
        assert_eq!(req.traceability[1].kind, TraceabilityKind::Specialization);
        assert_eq!(req.traceability[1].target.as_str(), "b");
    }

    fn get_requirement(
        input: &str,
    ) -> Requirement<String, node::expression::Arena<syntax::expression::Node>, syntax::markup::Node>
    {
        let nodes = parse(input).unwrap();
        let ast: Arena<_> = nodes.into_iter().map(syntax::entity::Node::from).collect();
        Vec::from(ast)
            .into_iter()
            .find_map(|n| n.variant.into_requirement())
            .expect("no requirement found")
    }
}
