use std::collections::HashMap;

use crate::{
    node::{self, entity::Idx},
    reference::Reference,
    semantics::link::entity::{init_imports, init_paths, iv_paths, link},
};

pub mod entity;
pub mod expression;
pub mod markup;

pub type Result<N> = std::result::Result<N, Reference>;

pub fn resolve<N, R: Into<Reference> + AsRef<str> + Clone, E, M>(
    arena: Vec<node::entity::Node<N, R, R, E, M>>,
) -> Vec<entity::Node<N, E, M>> {
    let mut paths = HashMap::with_capacity(arena.len());
    init_paths(&arena, Idx::default(), &Reference::default(), &mut paths);

    let mut parents = Vec::with_capacity(arena.len());
    init_imports(&arena, 0.into(), Vec::new(), &mut parents);

    let iv = iv_paths(&arena, &paths);
    link(&paths, iv, parents, arena)
}

pub fn paths<N, R: Into<Reference> + AsRef<str> + Clone, E, M>(
    arena: &Vec<node::entity::Node<N, R, R, E, M>>,
) -> Vec<Reference> {
    let mut paths = HashMap::with_capacity(arena.len());
    init_paths(&arena, Idx::default(), &Reference::default(), &mut paths);
    iv_paths(&arena, &paths)
        .into_iter()
        .map(|p| p[0].clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use crate::{
        mock,
        node::{entity as entity_node, expression as expr_node, markup},
        reference::Reference,
        semantics::link::{entity, expression, resolve},
        syntax,
    };

    fn get_child(arena: &[entity::Node<(), (), ()>], parent_idx: usize, label: &str) -> usize {
        arena[parent_idx]
            .variant
            .children()
            .expect("node has no children")
            .iter()
            .copied()
            .find(|idx| arena[usize::from(*idx)].meta.label == label)
            .map(usize::from)
            .unwrap_or_else(|| panic!("child '{}' not found", label))
    }

    fn assert_resolves_to(arena: &[entity::Node<(), (), ()>], idx: usize, label: &str) {
        let rf = arena[idx].variant.references().next().cloned().unwrap();
        assert!(rf.is_ok(), "expected resolved reference, got: {:?}", rf);
        assert_eq!(arena[usize::from(rf.unwrap())].meta.label, label);
    }

    #[test]
    fn resolves_complete_path() {
        let mut p = mock::entity::package("p");
        p.variant.children_mut().unwrap().push(1.into());
        p.variant.children_mut().unwrap().push(2.into());

        let x = mock::entity::attribute("x", "real");

        let mut r1 = mock::entity::requirement("r1");
        r1.variant.requirement_mut().unwrap().restriction = Some(Reference::new("p::x"));

        let resolved = resolve(vec![p, x, r1]);

        assert_resolves_to(&resolved, 2, "x");
    }

    #[test]
    fn resolves_parent_path() {
        let mut p = mock::entity::package("p");
        p.variant.children_mut().unwrap().push(1.into());
        p.variant.children_mut().unwrap().push(2.into());

        let x = mock::entity::attribute("x", "real");

        let mut r1 = mock::entity::requirement("r1");
        r1.variant.requirement_mut().unwrap().restriction = Some(Reference::new("x"));

        let resolved = resolve(vec![p, x, r1]);

        assert_resolves_to(&resolved, 2, "x");
    }

    #[test]
    fn resolves_parent_subpath() {
        let mut p = mock::entity::package("p");
        p.variant.children_mut().unwrap().push(1.into());
        p.variant.children_mut().unwrap().push(3.into());

        let mut y = mock::entity::part("y");
        y.variant.children_mut().unwrap().push(2.into());

        let x = mock::entity::attribute("x", "real");

        let mut r1 = mock::entity::requirement("r1");
        r1.variant.requirement_mut().unwrap().restriction = Some(Reference::new("y::x"));

        let resolved = resolve(vec![p, y, x, r1]);

        assert_resolves_to(&resolved, 3, "x");
    }

    #[test]
    fn resolves_import_subpath() {
        let mut p = mock::entity::package("p");
        let mut y = mock::entity::part("y");
        let x = mock::entity::attribute("x", "real");
        let mut q = mock::entity::package("q");
        let mut r1 = mock::entity::requirement("r1");

        p.variant.children_mut().unwrap().push(1.into());
        p.variant.children_mut().unwrap().push(3.into());
        y.variant.children_mut().unwrap().push(2.into());
        q.variant
            .imports_mut()
            .unwrap()
            .push(Reference::new("p::y"));
        q.variant.children_mut().unwrap().push(4.into());

        r1.variant.requirement_mut().unwrap().restriction = Some(Reference::new("x"));

        let resolved = resolve(vec![p, y, x, q, r1]);

        let r1_idx = get_child(&resolved, 3, "r1");
        assert_resolves_to(&resolved, r1_idx, "x");
    }

    #[test]
    fn resolves_attribute_domain_to_entity() {
        let mut p = mock::entity::package("p");
        p.variant.children_mut().unwrap().push(1.into());
        p.variant.children_mut().unwrap().push(2.into());

        let y = mock::entity::attribute("y", "real");
        let x = mock::entity::attribute("x", "p::y");

        let resolved = resolve(vec![p, y, x]);

        let x_idx = get_child(&resolved, 0, "x");
        let domain = resolved[x_idx].variant.expressions().next().unwrap();
        let target = match &domain[0].variant {
            expr_node::Variant::Identifier(ident) => ident.target.as_ref().unwrap(),
            _ => panic!("expected identifier"),
        };
        match target {
            expression::Target::Entity(entity_idx) => {
                assert_eq!(resolved[usize::from(*entity_idx)].meta.label, "y");
            }
            other => panic!("expected entity target, got {:?}", other),
        }
    }

    #[test]
    fn resolves_attribute_domain_to_builtin() {
        let mut p = mock::entity::package("p");
        p.variant.children_mut().unwrap().push(1.into());

        let x = mock::entity::attribute("x", "real");

        let resolved = resolve(vec![p, x]);

        let x_idx = get_child(&resolved, 0, "x");
        let domain = resolved[x_idx].variant.expressions().next().unwrap();
        let target = match &domain[0].variant {
            expr_node::Variant::Identifier(ident) => ident.target.as_ref().unwrap(),
            _ => panic!("expected identifier"),
        };
        assert!(
            matches!(target, expression::Target::Builtin(_)),
            "expected builtin target, got {:?}",
            target,
        );
    }

    #[test]
    fn resolves_forall_body_to_local_variable() {
        // Expression arena (pre-order): [forall(domain=Idx(1), body=Idx(2)), S, x]
        let set = expr_node::SetElement::new("x".to_string(), 1.into());
        let forall = expr_node::Forall::new(set, 2.into());
        let forall_node = syntax::expression::Node::empty(expr_node::Variant::Forall(forall));
        let domain_s = mock::expression::identifier("S");
        let body_x = mock::expression::identifier("x");
        let expr_arena = vec![forall_node, domain_s, body_x];

        let a = syntax::entity::Node::empty(
            entity_node::Meta::from("a".to_string()),
            entity_node::Variant::Attribute(entity_node::Attribute::new(expr_arena)),
        );

        let mut p = mock::entity::package("p");
        p.variant.children_mut().unwrap().push(1.into());

        let resolved = resolve(vec![p, a]);

        let a_idx = get_child(&resolved, 0, "a");
        let expr_arena = resolved[a_idx].variant.expressions().next().unwrap();

        let (body_idx, domain_idx) = match &expr_arena[0].variant {
            expr_node::Variant::Forall(f) => (usize::from(f.expression), usize::from(f.set.domain)),
            _ => panic!("expected forall"),
        };

        let target = match &expr_arena[body_idx].variant {
            expr_node::Variant::Identifier(ident) => ident.target.as_ref().unwrap(),
            _ => panic!("expected identifier"),
        };
        match target {
            expression::Target::Expression(_, idx) => {
                assert_eq!(usize::from(*idx), domain_idx);
            }
            other => panic!("expected local variable target, got {:?}", other),
        }
    }

    #[test]
    fn resolves_markup_reference() {
        let mut comment = markup::Container::default();
        comment
            .items
            .push(markup::Item::Reference(Reference::new("p")));

        let mut p = mock::entity::package("p");
        p.meta.comment = Some(markup::Node::empty(comment));

        let resolved = resolve(vec![p]);

        let markup = resolved[0].markups().next().cloned().unwrap();
        let rf = markup.container.references().next().cloned().unwrap();
        assert!(rf.is_ok());
        assert_eq!(resolved[usize::from(rf.unwrap())].meta.label, "p");
    }
}
