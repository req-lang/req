use std::fmt::Display;

use crate::{
    node::{self, entity::Idx},
    semantics::path::{self, Located, Path},
};

pub mod entity;
pub mod expression;
pub mod markup;

#[derive(Debug, Clone, thiserror::Error)]
pub enum Issue {
    #[error("Reference target does not exist")]
    NotFound,
    #[error("Reference target can points to several, diverging locations")]
    Ambiguous,
}

#[derive(Debug, Clone)]
pub struct Link<I = Idx> {
    pub original: Path,
    pub resolved: Result<I, Issue>,
}

impl<I> Link<I> {
    pub fn valid(original: Path, resolved: I) -> Self {
        Link {
            original,
            resolved: Ok(resolved),
        }
    }

    pub fn invalid(original: Path, resolved: Issue) -> Self {
        Link {
            original,
            resolved: Err(resolved),
        }
    }

    pub fn is_valid(&self) -> bool {
        self.resolved.is_ok()
    }

    pub fn target(&self) -> Option<&I> {
        match &self.resolved {
            Ok(i) => Some(i),
            Err(_) => None,
        }
    }
}

impl<I> From<Link<I>> for Path {
    fn from(value: Link<I>) -> Self {
        value.original
    }
}

impl<I> Display for Link<I> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.original)
    }
}

pub trait Resolve<N, E, M> {
    fn resolve(self) -> Vec<entity::Node<N, E, M>>;
}

impl<N: Located, R: Into<Path> + Clone, T: Into<Path> + Clone, E, M> Resolve<N, E, M>
    for Vec<node::entity::Node<N, R, T, E, M>>
{
    fn resolve(self) -> Vec<entity::Node<N, E, M>> {
        let paths = path::collect(&self)
            .into_iter()
            .map(|(p, mut idxs)| (p, idxs.pop().unwrap()))
            .collect();
        let mut imports = Vec::new();
        entity::collect_imports(&self, 0.into(), Vec::new(), &mut imports);
        entity::link(&paths, imports, self)
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use crate::{
        mock, node,
        semantics::{
            link::{
                Resolve,
                entity::{self, Node},
                expression::Target,
            },
            path::{self, Locate},
        },
        syntax,
    };

    fn resolve(arena: Vec<syntax::entity::Node>) -> Vec<Node<path::Location, (), ()>> {
        arena.locate().resolve()
    }

    fn get_child(
        arena: &[entity::Node<path::Location, (), ()>],
        parent_idx: usize,
        label: &str,
    ) -> usize {
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

    fn assert_resolves_to(arena: &[entity::Node<path::Location, (), ()>], idx: usize, label: &str) {
        let rf = arena[idx].variant.references().next().cloned().unwrap();
        assert!(rf.is_valid(), "expected resolved reference, got: {:?}", rf);
        assert_eq!(arena[usize::from(*rf.target().unwrap())].meta.label, label);
    }

    #[test]
    fn resolves_complete_path() {
        let mut p = mock::entity::package("p");
        p.variant.children_mut().unwrap().push(1.into());
        p.variant.children_mut().unwrap().push(2.into());

        let x = mock::entity::attribute("x", "real");

        let mut r1 = mock::entity::requirement("r1");
        r1.variant.requirement_mut().unwrap().restriction = Some("p::x".into());

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
        r1.variant.requirement_mut().unwrap().restriction = Some("x".into());

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
        r1.variant.requirement_mut().unwrap().restriction = Some("y::x".into());

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
        q.variant.imports_mut().unwrap().push("p::y".into());
        q.variant.children_mut().unwrap().push(4.into());

        r1.variant.requirement_mut().unwrap().restriction = Some("x".into());

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
            node::expression::Variant::Identifier(ident) => ident.target.target(),
            _ => panic!("expected identifier"),
        };

        match target {
            Some(Target::Entity(idx)) => {
                assert_eq!(resolved[usize::from(*idx)].meta.label, "y");
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
            node::expression::Variant::Identifier(ident) => ident.target.target(),
            _ => panic!("expected identifier"),
        };
        assert!(
            matches!(target.unwrap(), Target::Builtin(_)),
            "expected builtin target, got {:?}",
            target,
        );
    }

    #[test]
    fn resolves_forall_body_to_local_variable() {
        // Expression arena (pre-order): [forall(domain=Idx(1), body=Idx(2)), S, x]
        let set = node::expression::SetElement::new("x".to_string(), 1.into());
        let forall = node::expression::Forall::new(set, 2.into());
        let forall_node =
            syntax::expression::Node::empty(node::expression::Variant::Forall(forall));
        let domain_s = mock::expression::identifier("S");
        let body_x = mock::expression::identifier("x");
        let expr_arena = vec![forall_node, domain_s, body_x];

        let a = syntax::entity::Node::empty(
            node::entity::Meta::from("a".to_string()),
            node::entity::Variant::Attribute(node::entity::Attribute::new(expr_arena)),
        );

        let mut p = mock::entity::package("p");
        p.variant.children_mut().unwrap().push(1.into());

        let resolved = resolve(vec![p, a]);

        let a_idx = get_child(&resolved, 0, "a");
        let expr_arena = resolved[a_idx].variant.expressions().next().unwrap();

        let (body_idx, domain_idx) = match &expr_arena[0].variant {
            node::expression::Variant::Forall(f) => {
                (usize::from(f.expression), usize::from(f.set.domain))
            }
            _ => panic!("expected forall"),
        };

        let target = match &expr_arena[body_idx].variant {
            node::expression::Variant::Identifier(ident) => ident.target.target(),
            _ => panic!("expected identifier"),
        };
        match target {
            Some(Target::Expression(idx)) => {
                assert_eq!(usize::from(*idx), domain_idx);
            }
            other => panic!("expected local variable target, got {:?}", other),
        }
    }

    #[test]
    fn resolves_markup_reference() {
        let mut comment = node::markup::Container::default();
        comment
            .items
            .push(node::markup::Item::Reference("p".into()));

        let mut p = mock::entity::package("p");
        p.meta.comment = Some(node::markup::Node::empty(comment));

        let resolved = resolve(vec![p]);

        let markup = resolved[0].markups().next().cloned().unwrap();
        let rf = markup.container.references().next().cloned().unwrap();
        assert!(rf.is_valid());
        assert_eq!(resolved[usize::from(*rf.target().unwrap())].meta.label, "p");
    }
}
