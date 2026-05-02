use crate::{
    node::entity::Arena,
    semantics::{
        link::{self, entity::Node},
        typing::{
            entity::init_types,
            expression::{Issue, TypeKind},
        },
    },
};

pub mod entity;
pub mod evaluate;
pub mod expression;

pub type TypeResult = std::result::Result<TypeKind, Issue>;

pub trait Evaluate<N, M> {
    fn evaluate(self) -> Arena<Node<N, TypeResult, M>>;
}
impl<N, E, M> Evaluate<N, M> for Arena<link::entity::Node<N, E, M>> {
    fn evaluate(self) -> Arena<Node<N, TypeResult, M>> {
        let types = init_types(&self);
        entity::evaluate(types, self)
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
        let idx = children.find(|c| arena[**c].meta.label == *label).unwrap();
        &arena[*idx]
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
            attr.domain[0].payload,
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
            attr.domain[0].payload,
            Ok(TypeKind::Set(Box::new(TypeKind::Boolean)))
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
        assert_eq!(attr.domain[0].payload, Ok(TypeKind::Number));
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
        assert_eq!(expr[0].payload, Ok(TypeKind::Number));
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
        assert_eq!(expr[0].payload, Ok(TypeKind::Boolean));
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
        assert_eq!(expr[0].payload, Err(Issue::Poison));
        assert_eq!(expr[1].payload, Err(Issue::DomainNotFound));
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
            expr[0].payload,
            Err(Issue::UnexpectedType(TypeKind::Number, TypeKind::Boolean))
        );
    }

    #[test]
    fn forall_over_builtin_domain_has_boolean_type() {
        // forall x in real, true  — domain is builtin Set(Number), body is Boolean
        let mut p = package("p");

        let domain = identifier("real");
        let body = boolean(true);
        let set = node::expression::SetElement::new("x".to_string(), 1.into());
        let forall = node::expression::Forall::new(set, 2.into());
        let forall_expr = syntax::expression::Node::new((), Forall(forall));

        let req = formal("r", vec![forall_expr, domain, body]);
        p.variant.children_mut().unwrap().push(1.into());

        let typed = resolve_and_evaluate(vec![p, req].into());
        let r = get_child(&typed, 0.into(), "r");

        let expr = r.variant.expressions().next().unwrap().clone();
        assert_eq!(expr[0].payload, Ok(TypeKind::Boolean));
    }

    #[test]
    fn forall_with_non_boolean_body_is_type_error() {
        // forall x in real, 1  — body is Number, not Boolean

        let mut p = package("p");

        let domain = identifier("real");
        let body = number(1);
        let set = node::expression::SetElement::new("x".to_string(), 1.into());
        let forall = node::expression::Forall::new(set, 2.into());
        let forall_expr = syntax::expression::Node::new((), Forall(forall));

        let req = formal("r", vec![forall_expr, domain, body]);
        p.variant.children_mut().unwrap().push(1.into());

        let typed = resolve_and_evaluate(vec![p, req].into());
        let r = get_child(&typed, 0.into(), "r");

        let expr = r.variant.expressions().next().unwrap().clone();
        assert_eq!(
            expr[0].payload,
            Err(Issue::UnexpectedType(TypeKind::Boolean, TypeKind::Number))
        );
    }
}
