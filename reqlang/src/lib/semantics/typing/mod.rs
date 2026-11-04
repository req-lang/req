use crate::{
    node::{self, tree::Tree},
    semantics::{
        link::Links,
        typing::expression::{Issue, TypeKind},
    },
};

pub mod entity;
pub mod evaluate;
pub mod expression;

/// The type of every expression of the tree
pub fn evaluate(tree: &Tree, links: &Links) -> node::expression::Arena<Result> {
    let mut types = tree.expressions.parallel();
    entity::evaluate(tree, links, &mut types);
    types.into_iter().map(|t| t.unwrap()).collect()
}

pub type Result = std::result::Result<TypeKind, Issue>;

#[cfg(test)]
mod tests {
    use crate::{
        mock::{
            entity::{add, add_parent, attribute, formal, package, typ},
            expression::{binary, boolean, identifier, insert, number},
        },
        node::{
            self,
            entity::Idx,
            expression::{
                Aggregator,
                BinaryOperator::*,
                Comprehension, SetElement,
                Variant::{Comprehension as Comprehension_, Set},
            },
            tree::{Draft, Tree},
        },
        semantics::{
            link::link,
            path::paths,
            typing::{
                Result, evaluate,
                expression::{Issue, TypeKind},
            },
        },
    };

    fn typed(draft: Draft) -> (Tree, node::expression::Arena<Result>) {
        let tree = Tree::from(draft);
        let links = link(&tree, &paths(&tree.entities));
        let types = evaluate(&tree, &links);
        (tree, types)
    }

    fn type_of(tree: &Tree, types: &node::expression::Arena<Result>, entity: Idx) -> Result {
        let root = tree.entities[entity].variant.expressions().next().unwrap();
        types[root].clone()
    }

    fn add_attribute(draft: &mut Draft, label: &str, domain: &str) -> Idx {
        let attr = attribute(draft, label, domain);
        add(draft, attr)
    }

    fn add_formal(draft: &mut Draft, root: node::expression::Idx) -> Idx {
        add(draft, formal("r", root))
    }

    #[test]
    fn attribute_with_real_domain_has_set_number_type() {
        let mut draft = Draft::default();
        let x = add_attribute(&mut draft, "x", "real");
        add_parent(&mut draft, package("p"), &[x]);

        let (tree, types) = typed(draft);

        assert_eq!(
            type_of(&tree, &types, x),
            Ok(TypeKind::Set(Box::new(TypeKind::Number)))
        );
    }

    #[test]
    fn attribute_with_boolean_domain_has_set_boolean_type() {
        let mut draft = Draft::default();
        let x = add_attribute(&mut draft, "x", "boolean");
        add_parent(&mut draft, package("p"), &[x]);

        let (tree, types) = typed(draft);

        assert_eq!(
            type_of(&tree, &types, x),
            Ok(TypeKind::Set(Box::new(TypeKind::Boolean)))
        );
    }

    #[test]
    fn attribute_with_enum_boolean_has_custom_type() {
        let mut draft = Draft::default();
        let t = add(&mut draft, typ("t", &["a", "b", "c"]));
        let x = add_attribute(&mut draft, "x", "t");
        add_parent(&mut draft, package("p"), &[t, x]);

        let (tree, types) = typed(draft);

        assert_eq!(type_of(&tree, &types, x), Ok(TypeKind::Custom(t)));
    }

    #[test]
    fn attribute_referencing_attribute_has_element_type() {
        // x: real => Set(Number); y: x => y's domain type is Number (element of x's set)
        let mut draft = Draft::default();
        let c0 = add_attribute(&mut draft, "x", "real");
        let y = add_attribute(&mut draft, "y", "x");
        add_parent(&mut draft, package("p"), &[c0, y]);

        let (tree, types) = typed(draft);

        assert_eq!(type_of(&tree, &types, y), Ok(TypeKind::Number));
    }

    #[test]
    fn formal_requirement_with_enum_has_enum_type() {
        let mut draft = Draft::default();
        let t = add(&mut draft, typ("t", &["a", "b", "c"]));
        let expr = identifier(&mut draft, "t::a");
        let r = add_formal(&mut draft, expr);
        add_parent(&mut draft, package("p"), &[t, r]);

        let (tree, types) = typed(draft);

        assert_eq!(type_of(&tree, &types, r), Ok(TypeKind::Custom(t)));
    }

    #[test]
    fn formal_requirement_with_sum_has_number_type() {
        let mut draft = Draft::default();
        let (n1, n2) = (number(&mut draft, 1), number(&mut draft, 2));
        let expr = binary(&mut draft, Plus, n1, n2);
        let r = add_formal(&mut draft, expr);
        add_parent(&mut draft, package("p"), &[r]);

        let (tree, types) = typed(draft);

        assert_eq!(type_of(&tree, &types, r), Ok(TypeKind::Number));
    }

    #[test]
    fn formal_requirement_with_comparison_has_boolean_type() {
        let mut draft = Draft::default();
        let (n1, n2) = (number(&mut draft, 1), number(&mut draft, 2));
        let expr = binary(&mut draft, GreaterThan, n1, n2);
        let r = add_formal(&mut draft, expr);
        add_parent(&mut draft, package("p"), &[r]);

        let (tree, types) = typed(draft);

        assert_eq!(type_of(&tree, &types, r), Ok(TypeKind::Boolean));
    }

    #[test]
    fn formal_requirement_with_unknown_reference() {
        let mut draft = Draft::default();
        let (n1, n2) = (identifier(&mut draft, "x"), number(&mut draft, 10));
        let expr = binary(&mut draft, Plus, n1, n2);
        let r = add_formal(&mut draft, expr);
        add_parent(&mut draft, package("p"), &[r]);

        let (tree, types) = typed(draft);

        assert_eq!(type_of(&tree, &types, r), Err(Issue::Poison));
        assert_eq!(types[n1], Err(Issue::Poison));
    }

    #[test]
    fn formal_requirement_with_mismatched_operands_is_type_error() {
        // Number + Boolean is not valid; expect UnexpectedType on the rhs
        let mut draft = Draft::default();
        let (n1, n2) = (number(&mut draft, 1), boolean(&mut draft, true));
        let expr = binary(&mut draft, Plus, n1, n2);
        let r = add_formal(&mut draft, expr);
        add_parent(&mut draft, package("p"), &[r]);

        let (tree, types) = typed(draft);

        assert_eq!(
            type_of(&tree, &types, r),
            Err(Issue::UnexpectedType(TypeKind::Number, TypeKind::Boolean))
        );
    }

    #[test]
    fn all_over_builtin_domain_has_boolean_type() {
        // all true for x in real end — domain is builtin Set(Number), body is Boolean
        let mut draft = Draft::default();

        let domain = identifier(&mut draft, "real");
        let body = boolean(&mut draft, true);
        let set = SetElement::new("x".to_string(), domain);
        let all = Comprehension::new(Some(Aggregator::All), set, body);
        let root = insert(&mut draft, Comprehension_(all));
        let r = add_formal(&mut draft, root);
        add_parent(&mut draft, package("p"), &[r]);

        let (tree, types) = typed(draft);

        assert_eq!(type_of(&tree, &types, r), Ok(TypeKind::Boolean));
    }

    #[test]
    fn nested_all_variables_have_number_type() {
        // all (all y > 1 for y in {x} end) for x in real end
        let mut draft = Draft::default();

        let real = identifier(&mut draft, "real");
        let x_element = identifier(&mut draft, "x");
        let set_x = insert(
            &mut draft,
            Set(node::expression::Set {
                elements: vec![x_element],
            }),
        );
        let y = identifier(&mut draft, "y");
        let x = identifier(&mut draft, "x");
        let body = binary(&mut draft, GreaterThan, y, x);
        let inner = Comprehension::new(
            Some(Aggregator::All),
            SetElement::new("y".to_string(), set_x),
            body,
        );
        let inner = insert(&mut draft, Comprehension_(inner));
        let outer = Comprehension::new(
            Some(Aggregator::All),
            SetElement::new("x".to_string(), real),
            inner,
        );
        let outer = insert(&mut draft, Comprehension_(outer));
        let c0 = add_formal(&mut draft, outer);
        add_parent(&mut draft, package("p"), &[c0]);

        let (_, types) = typed(draft);

        assert_eq!(types[x_element], Ok(TypeKind::Number));
        assert_eq!(types[y], Ok(TypeKind::Number));
        assert_eq!(types[x], Ok(TypeKind::Number));
    }

    #[test]
    fn all_with_non_boolean_body_is_type_error() {
        // all 1 for x in real end — body is Number, not Boolean
        let mut draft = Draft::default();

        let domain = identifier(&mut draft, "real");
        let body = number(&mut draft, 1);
        let set = SetElement::new("x".to_string(), domain);
        let all = Comprehension::new(Some(Aggregator::All), set, body);
        let root = insert(&mut draft, Comprehension_(all));
        let r = add_formal(&mut draft, root);
        add_parent(&mut draft, package("p"), &[r]);

        let (tree, types) = typed(draft);

        assert_eq!(
            type_of(&tree, &types, r),
            Err(Issue::UnexpectedType(TypeKind::Boolean, TypeKind::Number))
        );
    }
}
