use crate::{
    node::{self, entity::Arena},
    semantics::{
        link::{self, entity::Node},
        typing::expression::{Issue, TypeKind},
    },
};

pub mod entity;
pub mod evaluate;
pub mod expression;

pub type TypeResult<I = node::entity::Idx> = std::result::Result<TypeKind<I>, Issue>;

pub trait Evaluate<N, M> {
    fn evaluate(self) -> Arena<Node<N, TypeResult, M>>;
}
impl<N, E, M> Evaluate<N, M> for Arena<link::entity::Node<N, E, M>> {
    fn evaluate(self) -> Arena<Node<N, TypeResult, M>> {
        entity::evaluate(self)
    }
}

pub trait Typed {
    fn type_result(&self) -> &TypeResult;
}

impl Typed for TypeResult {
    fn type_result(&self) -> &TypeResult {
        self
    }
}

#[cfg(test)]
mod tests {

    use crate::{
        mock::{entity::*, expression::*},
        node::{
            self,
            entity::{Arena, Idx},
            expression::{BinaryOperator::*, Variant::*},
        },
        semantics::{
            link::{self, Resolve},
            path::{self, Locate},
            typing::{
                Evaluate, TypeResult,
                expression::{Issue, TypeKind},
            },
        },
        syntax,
        utils::children::{ChildrenIter, ChildrenMut},
    };

    type Node = link::entity::Node<path::Location, TypeResult, ()>;

    fn resolve_and_evaluate(syn: Arena<syntax::entity::Node>) -> Arena<Node> {
        syn.locate().resolve().evaluate()
    }

    fn get_child<'a>(arena: &'a Arena<Node>, parent: Idx, label: &str) -> &'a Node {
        let parent = &arena[parent];
        let mut children = parent.variant.children_iter();
        let idx = children.find(|c| arena[*c].meta.label == *label).unwrap();
        &arena[idx]
    }

    #[test]
    fn attribute_with_real_domain_has_set_number_type() {
        let mut p = package("p");
        let x = attribute("x", "real");
        p.variant.children_mut().unwrap().push(1.into());

        let typed = resolve_and_evaluate(vec![p, x].into());
        let x = get_child(&typed, 0.into(), "x");

        let attr = x.variant.attribute().unwrap();
        assert_eq!(
            attr.domain[0.into()].payload,
            Ok(TypeKind::Set(Box::new(TypeKind::Number)))
        );
    }

    #[test]
    fn attribute_with_boolean_domain_has_set_boolean_type() {
        let mut p = package("p");
        let x = attribute("x", "boolean");
        p.variant.children_mut().unwrap().push(1.into());

        let typed = resolve_and_evaluate(vec![p, x].into());
        let x = get_child(&typed, 0.into(), "x");

        let attr = x.variant.attribute().unwrap();
        assert_eq!(
            attr.domain[0.into()].payload,
            Ok(TypeKind::Set(Box::new(TypeKind::Boolean)))
        );
    }

    #[test]
    fn attribute_with_enum_boolean_has_custom_type() {
        let mut p = package("p");
        let t = typ("t", &["a", "b", "c"]);
        let x = attribute("x", "t");
        p.variant
            .children_mut()
            .unwrap()
            .extend(&[1.into(), 2.into()]);

        let typed = resolve_and_evaluate(vec![p, t, x].into());
        let x = get_child(&typed, 0.into(), "x");

        let attr = x.variant.attribute().unwrap();
        assert_eq!(
            attr.domain[0.into()].payload,
            Ok(TypeKind::Custom(1.into()))
        );
    }

    #[test]
    fn attribute_referencing_attribute_has_element_type() {
        // x: real => Set(Number); y: x => y's domain type is Number (element of x's set)
        // SHOULD FAIL
        let mut p = package("p");
        let x = attribute("x", "real");
        let y = attribute("y", "x");
        p.variant.children_mut().unwrap().push(1.into());
        p.variant.children_mut().unwrap().push(2.into());

        let typed = resolve_and_evaluate(vec![p, x, y].into());
        let y = get_child(&typed, 0.into(), "y");

        let attr = y.variant.attribute().unwrap();
        assert_eq!(attr.domain[0.into()].payload, Ok(TypeKind::Number));
    }

    #[test]
    fn formal_requirement_with_enum_has_enum_type() {
        let mut p = package("p");
        let t = typ("t", &["a", "b", "c"]);

        let req = formal("r", vec![identifier("t::a")]);
        p.variant
            .children_mut()
            .unwrap()
            .extend(&[1.into(), 2.into()]);

        let typed = resolve_and_evaluate(vec![p, t, req].into());
        let r = get_child(&typed, 0.into(), "r");

        let expr = r.variant.expressions().next().unwrap().clone();
        assert_eq!(expr[0.into()].payload, Ok(TypeKind::Custom(1.into())));
    }

    #[test]
    fn formal_requirement_with_sum_has_number_type() {
        let mut p = package("p");
        let (n1, n2) = (number(1), number(2));
        let expr = binary(Plus, 1, 2);
        let req = formal("r", vec![expr, n1, n2]);
        p.variant.children_mut().unwrap().push(1.into());

        let typed = resolve_and_evaluate(vec![p, req].into());
        let r = get_child(&typed, 0.into(), "r");

        let expr = r.variant.expressions().next().unwrap().clone();
        assert_eq!(expr[0.into()].payload, Ok(TypeKind::Number));
    }

    #[test]
    fn formal_requirement_with_comparison_has_boolean_type() {
        let mut p = package("p");
        let (n1, n2) = (number(1), number(2));
        let expr = binary(GreaterThan, 1, 2);
        let req = formal("r", vec![expr, n1, n2]);
        p.variant.children_mut().unwrap().push(1.into());

        let typed = resolve_and_evaluate(vec![p, req].into());
        let r = get_child(&typed, 0.into(), "r");

        let expr = r.variant.expressions().next().unwrap().clone();
        assert_eq!(expr[0.into()].payload, Ok(TypeKind::Boolean));
    }

    #[test]
    fn formal_requirement_with_unknown_reference() {
        // Number + Boolean is not valid; expect UnexpectedType on the rhs
        let mut p = package("p");
        let (n1, n2) = (identifier("x"), number(10));
        let expr = binary(Plus, 1, 2);
        let req = formal("r", vec![expr, n1, n2]);
        p.variant.children_mut().unwrap().push(1.into());

        let typed = resolve_and_evaluate(vec![p, req].into());
        let r = get_child(&typed, 0.into(), "r");

        let expr = r.variant.expressions().next().unwrap().clone();
        assert_eq!(expr[0.into()].payload, Err(Issue::Poison));
        assert_eq!(expr[1.into()].payload, Err(Issue::Poison));
    }

    #[test]
    fn formal_requirement_with_mismatched_operands_is_type_error() {
        // Number + Boolean is not valid; expect UnexpectedType on the rhs
        let mut p = package("p");
        let (n1, n2) = (number(1), boolean(true));
        let expr = binary(Plus, 1, 2);
        let req = formal("r", vec![expr, n1, n2]);
        p.variant.children_mut().unwrap().push(1.into());

        let typed = resolve_and_evaluate(vec![p, req].into());
        let r = get_child(&typed, 0.into(), "r");

        let expr = r.variant.expressions().next().unwrap().clone();
        assert_eq!(
            expr[0.into()].payload,
            Err(Issue::UnexpectedType(TypeKind::Number, TypeKind::Boolean))
        );
    }

    #[test]
    fn all_over_builtin_domain_has_boolean_type() {
        // all true for x in real end — domain is builtin Set(Number), body is Boolean
        let mut p = package("p");

        let domain = identifier("real");
        let body = boolean(true);
        let set = node::expression::SetElement::new("x".to_string(), 1.into());
        let all = node::expression::Comprehension::new(Some(node::expression::Aggregator::All), set, 2.into());
        let all_expr = syntax::expression::Node::new((), Comprehension(all));

        let req = formal("r", vec![all_expr, domain, body]);
        p.variant.children_mut().unwrap().push(1.into());

        let typed = resolve_and_evaluate(vec![p, req].into());
        let r = get_child(&typed, 0.into(), "r");

        let expr = r.variant.expressions().next().unwrap().clone();
        assert_eq!(expr[0.into()].payload, Ok(TypeKind::Boolean));
    }

    #[test]
    fn nested_all_variables_have_number_type() {
        // all (all y > 1 for y in {x} end) for x in real end
        let mut p = package("p");

        let outer_set = node::expression::SetElement::new("x".to_string(), 1.into());
        let outer_all = syntax::expression::Node::new(
            (),
            Comprehension(node::expression::Comprehension::new(
                Some(node::expression::Aggregator::All),
                outer_set,
                2.into(),
            )),
        );

        let inner_set = node::expression::SetElement::new("y".to_string(), 3.into());
        let inner_all = syntax::expression::Node::new(
            (),
            Comprehension(node::expression::Comprehension::new(
                Some(node::expression::Aggregator::All),
                inner_set,
                5.into(),
            )),
        );

        let set_x = syntax::expression::Node::new(
            (),
            Set(node::expression::Set {
                elements: vec![4.into()],
            }),
        );

        let req = formal(
            "r",
            vec![
                outer_all,                 // 0: outer all
                identifier("real"),        // 1: domain of x
                inner_all,                 // 2: inner all (body of outer)
                set_x,                     // 3: {x} (domain of y)
                identifier("x"),           // 4: x (element of {x})
                binary(GreaterThan, 6, 7), // 5: y > 1 (body of inner)
                identifier("y"),           // 6: y
                identifier("x"),           // 7: 1
            ],
        );
        p.variant.children_mut().unwrap().push(1.into());

        let typed = resolve_and_evaluate(vec![p, req].into());
        let r = get_child(&typed, 0.into(), "r");

        let expr = r.variant.expressions().next().unwrap().clone();
        assert_eq!(expr[4.into()].payload, Ok(TypeKind::Number)); // x is Number
        assert_eq!(expr[6.into()].payload, Ok(TypeKind::Number)); // y is Number
        assert_eq!(expr[7.into()].payload, Ok(TypeKind::Number)); // y is Number
    }

    #[test]
    fn all_with_non_boolean_body_is_type_error() {
        // all 1 for x in real end — body is Number, not Boolean

        let mut p = package("p");

        let domain = identifier("real");
        let body = number(1);
        let set = node::expression::SetElement::new("x".to_string(), 1.into());
        let all = node::expression::Comprehension::new(Some(node::expression::Aggregator::All), set, 2.into());
        let all_expr = syntax::expression::Node::new((), Comprehension(all));

        let req = formal("r", vec![all_expr, domain, body]);
        p.variant.children_mut().unwrap().push(1.into());

        let typed = resolve_and_evaluate(vec![p, req].into());
        let r = get_child(&typed, 0.into(), "r");

        let expr = r.variant.expressions().next().unwrap().clone();
        assert_eq!(
            expr[0.into()].payload,
            Err(Issue::UnexpectedType(TypeKind::Boolean, TypeKind::Number))
        );
    }
}
