use std::collections::HashMap;

use crate::{
    node::NodeRef,
    semantics::{
        reference,
        typing::entity::{Map, Node, build_entity_map, init_node},
    },
};

pub mod entity;
pub mod expression;
pub mod markup;

pub fn evaluate(node: NodeRef<reference::entity::Node>) -> NodeRef<Node> {
    let mut entities: Map = HashMap::new();
    let mut expressions: expression::Map = HashMap::new();
    build_entity_map(&node, &mut entities);
    let node = init_node(node, &entities, &mut expressions);
    entity::evaluate(node.clone());
    node
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use crate::{
        mock::{entity::*, expression::*},
        node::{self, NodeRef, expression::BinaryOperator::*, expression::Variant::*},
        semantics::{
            reference,
            typing::{
                entity, evaluate,
                expression::{Issue, TypeKind},
            },
        },
        syntax,
        utils::children::{ChildrenIter, ChildrenMut},
    };

    fn resolve_and_evaluate(syn: NodeRef<syntax::entity::Node>) -> NodeRef<entity::Node> {
        let resolved = reference::resolve(syn);
        let typed = evaluate(resolved);
        typed
    }

    fn get_child(parent: &NodeRef<entity::Node>, label: &str) -> NodeRef<entity::Node> {
        parent
            .borrow()
            .variant
            .children_iter()
            .find(|n| n.borrow().variant.meta().label == label)
            .unwrap()
    }

    #[test]
    fn attribute_with_real_domain_has_set_number_type() {
        let p = package("p");
        p.borrow_mut()
            .variant
            .children_mut()
            .unwrap()
            .push(attribute("x", "real"));

        let typed = resolve_and_evaluate(p);
        let x = get_child(&typed, "x");

        let x_borrow = x.borrow();
        let attr = x_borrow.variant.attribute().unwrap();
        assert_eq!(
            attr.domain.borrow().typ,
            Ok(TypeKind::Set(Box::new(TypeKind::Number)))
        );
    }

    #[test]
    fn attribute_with_boolean_domain_has_set_boolean_type() {
        let p = package("p");
        p.borrow_mut()
            .variant
            .children_mut()
            .unwrap()
            .push(attribute("x", "boolean"));

        let typed = resolve_and_evaluate(p);
        let x = get_child(&typed, "x");

        let x_borrow = x.borrow();
        let attr = x_borrow.variant.attribute().unwrap();
        assert_eq!(
            attr.domain.borrow().typ,
            Ok(TypeKind::Set(Box::new(TypeKind::Boolean)))
        );
    }

    #[test]
    fn attribute_referencing_attribute_has_element_type() {
        // x: real => Set(Number); y: x => y's domain type is Number (element of x's set)
        let p = package("p");
        {
            let mut p_mut = p.borrow_mut();
            let children = p_mut.variant.children_mut().unwrap();
            children.push(attribute("x", "real"));
            children.push(attribute("y", "x"));
        }

        let typed = resolve_and_evaluate(p);
        let y = get_child(&typed, "y");

        let y_borrow = y.borrow();
        let attr = y_borrow.variant.attribute().unwrap();
        assert_eq!(attr.domain.borrow().typ, Ok(TypeKind::Number));
    }

    #[test]
    fn formal_requirement_with_sum_has_number_type() {
        let p = package("p");
        let expr = binary(Plus, number(1), number(2));
        p.borrow_mut()
            .variant
            .children_mut()
            .unwrap()
            .push(formal("r", expr));

        let typed = resolve_and_evaluate(p);
        let r = get_child(&typed, "r");

        let r_borrow = r.borrow();
        let expr = r_borrow.variant.expressions().next().unwrap().clone();
        assert_eq!(expr.borrow().typ, Ok(TypeKind::Number));
    }

    #[test]
    fn formal_requirement_with_comparison_has_boolean_type() {
        let p = package("p");
        let expr = binary(GreaterThan, number(1), number(0));
        p.borrow_mut()
            .variant
            .children_mut()
            .unwrap()
            .push(formal("r", expr));

        let typed = resolve_and_evaluate(p);
        let r = get_child(&typed, "r");

        let r_borrow = r.borrow();
        let expr = r_borrow.variant.expressions().next().unwrap().clone();
        assert_eq!(expr.borrow().typ, Ok(TypeKind::Boolean));
    }

    #[test]
    fn formal_requirement_with_unknown_reference() {
        // Number + Boolean is not valid; expect UnexpectedType on the rhs
        let p = package("p");
        let expr = binary(Plus, identifier("x"), number(10));
        p.borrow_mut()
            .variant
            .children_mut()
            .unwrap()
            .push(formal("r", expr));

        let typed = resolve_and_evaluate(p);
        let r = get_child(&typed, "r");

        let r_borrow = r.borrow();
        let expr = r_borrow.variant.expressions().next().unwrap().clone();
        let child = expr.borrow().variant.children_iter().next().unwrap();
        let child_typ = &child.borrow().typ;

        assert_eq!(expr.borrow().typ, Err(Issue::Poison));
        assert_eq!(*child_typ, Err(Issue::DomainNotFound));
    }

    #[test]
    fn formal_requirement_with_mismatched_operands_is_type_error() {
        // Number + Boolean is not valid; expect UnexpectedType on the rhs
        let p = package("p");
        let expr = binary(Plus, number(1), boolean(true));
        p.borrow_mut()
            .variant
            .children_mut()
            .unwrap()
            .push(formal("r", expr));

        let typed = resolve_and_evaluate(p);
        let r = get_child(&typed, "r");

        let r_borrow = r.borrow();
        let expr = r_borrow.variant.expressions().next().unwrap().clone();
        assert_eq!(
            expr.borrow().typ,
            Err(Issue::UnexpectedType(TypeKind::Number, TypeKind::Boolean))
        );
    }

    #[test]
    fn forall_over_builtin_domain_has_boolean_type() {
        // forall x in real, true  — domain is builtin Set(Number), body is Boolean
        let p = package("p");

        let domain = identifier("real");
        let body = boolean(true);
        let set = node::expression::SetElement::new("x".to_string(), domain);
        let forall = node::expression::Forall::new(set, body);
        let forall_expr = node::new(syntax::expression::Node::new(Forall(forall)));

        p.borrow_mut()
            .variant
            .children_mut()
            .unwrap()
            .push(formal("r", forall_expr));

        let typed = resolve_and_evaluate(p);
        let r = get_child(&typed, "r");

        let r_borrow = r.borrow();
        let expr = r_borrow.variant.expressions().next().unwrap().clone();
        assert_eq!(expr.borrow().typ, Ok(TypeKind::Boolean));
    }

    #[test]
    fn forall_with_non_boolean_body_is_type_error() {
        // forall x in real, 1  — body is Number, not Boolean
        let p = package("p");

        let domain = identifier("real");
        let body = number(1);
        let set = node::expression::SetElement::new("x".to_string(), domain);
        let forall = node::expression::Forall::new(set, body);
        let forall_expr = node::new(syntax::expression::Node::new(Forall(forall)));

        p.borrow_mut()
            .variant
            .children_mut()
            .unwrap()
            .push(formal("r", forall_expr));

        let typed = resolve_and_evaluate(p);
        let r = get_child(&typed, "r");

        let r_borrow = r.borrow();
        let expr = r_borrow.variant.expressions().next().unwrap().clone();
        assert_eq!(
            expr.borrow().typ,
            Err(Issue::UnexpectedType(TypeKind::Boolean, TypeKind::Number))
        );
    }
}
