use std::{collections::HashMap, fmt::Display};

use crate::{
    node::{
        self,
        entity::{Arena, Idx},
    },
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
    pub resolved: Option<I>,
}

impl<I> Link<I> {
    pub fn valid(original: Path, resolved: I) -> Self {
        Link {
            original,
            resolved: Some(resolved),
        }
    }

    pub fn invalid(original: Path) -> Self {
        Link {
            original,
            resolved: None,
        }
    }

    pub fn is_valid(&self) -> bool {
        self.resolved.is_some()
    }

    pub fn target(&self) -> Option<&I> {
        match &self.resolved {
            Some(i) => Some(i),
            None => None,
        }
    }

    pub fn target_mut(&mut self) -> Option<&mut I> {
        match &mut self.resolved {
            Some(i) => Some(i),
            None => None,
        }
    }

    pub fn map<J>(self, f: impl FnOnce(I) -> J) -> Link<J> {
        Link {
            original: self.original,
            resolved: self.resolved.map(f),
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
    fn resolve(self) -> Arena<entity::Node<N, E, M>>;
}

impl<N: Located, R: Into<Path> + Clone, T: Into<Path> + Clone, E, M> Resolve<N, E, M>
    for Arena<node::entity::Node<N, R, T, E, M>>
{
    fn resolve(self) -> Arena<entity::Node<N, E, M>> {
        let paths = path::collect(&self)
            .into_iter()
            .map(|(p, mut idxs)| (p, idxs.pop().unwrap()))
            .collect();
        let annotations = annotate_types(&self);
        entity::link(&paths, &annotations, self)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum BuiltinKind {
    Real,
    Integer,
    Boolean,
}

impl BuiltinKind {
    fn find<S: AsRef<str>>(value: S) -> Option<Self> {
        use BuiltinKind::*;

        match value.as_ref() {
            "real" => Some(Real),
            "integer" => Some(Integer),
            "boolean" => Some(Boolean),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            BuiltinKind::Real => "real",
            BuiltinKind::Integer => "integer",
            BuiltinKind::Boolean => "boolean",
        }
    }
}

#[derive(Debug, Clone)]
pub enum Target<I = node::entity::Idx> {
    Entity(I),
    Builtin(BuiltinKind),
    Enum(I, usize),
}

impl<I> Target<I> {
    pub fn entity(&self) -> Option<I>
    where
        I: Copy,
    {
        match self {
            Target::Entity(rf) => Some(*rf),
            _ => None,
        }
    }

    pub fn entity_mut(&mut self) -> Option<&mut I>
    where
        I: Copy,
    {
        match self {
            Target::Entity(rf) => Some(rf),
            _ => None,
        }
    }
}

impl Target<Idx> {
    pub fn find(ctx: &FindCtx, rf: &Path) -> Option<Self> {
        let parent = rf.parent().unwrap_or(rf.clone());
        let label = rf.label();
        if let Some(idx) = entity::find(ctx, &parent)
            && let Some(pos) = ctx.annotations[idx]
                .enum_members
                .iter()
                .position(|s| *s == label)
        {
            return Some(Self::Enum(idx, pos));
        }

        if let Some(idx) = entity::find(ctx, rf) {
            return Some(Self::Entity(idx));
        }

        if let Some(builtin) = BuiltinKind::find(rf.as_str()) {
            return Some(Self::Builtin(builtin));
        }

        return None;
    }
}

#[derive(Debug, Default)]
pub(super) struct Annotation {
    enum_members: Vec<String>,
}

pub(super) fn annotate_types<N, R, T, E, M>(
    arena: &Arena<node::entity::Node<N, R, T, E, M>>,
) -> Arena<Annotation> {
    (&arena)
        .into_iter()
        .map(|node| {
            let enum_members = match &node.variant {
                node::entity::Variant::Type(typ) => typ.members.clone(),
                _ => Vec::new(),
            };
            Annotation { enum_members }
        })
        .collect()
}

pub struct FindCtx<'a> {
    nodes: &'a HashMap<Path, Idx>,
    annotations: &'a Arena<Annotation>,
    current: &'a Path,
    imports: Vec<Path>,
}

impl<'a> FindCtx<'a> {
    fn new(
        nodes: &'a HashMap<Path, Idx>,
        annotations: &'a Arena<Annotation>,
        current: &'a Path,
        imports: Vec<Path>,
    ) -> Self {
        Self {
            nodes,
            annotations,
            current,
            imports,
        }
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use crate::{
        mock,
        node::{
            self,
            entity::{Arena, Idx, RequirementVariant},
        },
        semantics::{
            link::{
                Resolve,
                entity::{self, Node},
                expression::Target,
            },
            path::{self, Locate},
        },
        syntax,
        utils::children::{Children, ChildrenMut},
    };

    fn resolve(arena: Vec<syntax::entity::Node>) -> Arena<Node<path::Location, (), ()>> {
        Arena::from(arena).locate().resolve()
    }

    fn get_child(
        arena: &Arena<entity::Node<path::Location, (), ()>>,
        parent_idx: Idx,
        label: &str,
    ) -> Idx {
        arena[parent_idx]
            .variant
            .children()
            .expect("node has no children")
            .iter()
            .copied()
            .find(|idx| arena[*idx].meta.label == label)
            .unwrap_or_else(|| panic!("child '{}' not found", label))
    }

    fn assert_resolves_to(
        arena: &Arena<entity::Node<path::Location, (), ()>>,
        idx: Idx,
        label: &str,
    ) {
        let rf = arena[idx].variant.references().next().cloned().unwrap();
        assert!(rf.is_valid(), "expected resolved reference, got: {:?}", rf);
        assert_eq!(arena[*rf.target().unwrap()].meta.label, label);
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

        assert_resolves_to(&resolved, 2.into(), "x");
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

        assert_resolves_to(&resolved, 2.into(), "x");
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

        assert_resolves_to(&resolved, 3.into(), "x");
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

        let r1_idx = get_child(&resolved, 3.into(), "r1");
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

        let x_idx = get_child(&resolved, 0.into(), "x");
        let domain = resolved[x_idx].variant.expressions().next().unwrap();
        let target = match &domain[0.into()].variant {
            node::expression::Variant::Identifier(ident) => ident.target.target(),
            _ => panic!("expected identifier"),
        };

        match target {
            Some(Target::Outer(super::Target::Entity(idx))) => {
                assert_eq!(resolved[*idx].meta.label, "y");
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

        let x_idx = get_child(&resolved, 0.into(), "x");
        let domain = resolved[x_idx].variant.expressions().next().unwrap();
        let target = match &domain[0.into()].variant {
            node::expression::Variant::Identifier(ident) => ident.target.target(),
            _ => panic!("expected identifier"),
        };
        assert!(
            matches!(target.unwrap(), Target::Outer(super::Target::Builtin(_))),
            "expected builtin target, got {:?}",
            target,
        );
    }

    #[test]
    fn resolves_type_enum() {
        let mut p = mock::entity::package("p");
        p.variant
            .children_mut()
            .unwrap()
            .extend(&[1.into(), 2.into(), 3.into()]);

        let t = mock::entity::typ("t", &["a", "b", "c"]);
        let x = mock::entity::attribute("x", "t");
        let mut r1 = mock::entity::requirement("r1");

        let lhs = mock::expression::identifier("x");
        let rhs = mock::expression::identifier("t::a");
        let bin = mock::expression::binary(node::expression::BinaryOperator::Equal, 1, 2);
        let expr = vec![bin, lhs, rhs];
        r1.variant.requirement_mut().unwrap().variant = RequirementVariant::Formal(expr.into());

        let resolved = resolve(vec![p, t, x, r1]);

        let r1 = resolved[3.into()].variant.requirement().unwrap();
        let expr = r1.variant.formal().unwrap();
        let tgt = &expr[2.into()].variant.identifier().unwrap().target;
        assert!(tgt.is_valid());
        if let Target::Outer(super::Target::Enum(idx, 0)) = tgt.resolved.clone().unwrap() {
            assert_eq!(idx, 1.into());
            return;
        }
        panic!("unexpected target type, {:?}", tgt);
    }

    #[test]
    fn resolves_comprehension_body_to_local_variable() {
        // Expression arena (pre-order): [all body for x in domain, S, x]
        let set = node::expression::SetElement::new("x".to_string(), 1.into());
        let comprehension = node::expression::Comprehension::new(Some(node::expression::Aggregator::All), set, 2.into());
        let comprehension_node = syntax::expression::Node::empty(
            node::expression::Variant::Comprehension(comprehension),
        );
        let domain_s = mock::expression::identifier("S");
        let body_x = mock::expression::identifier("x");
        let expr_arena = vec![comprehension_node, domain_s, body_x].into();

        let a = syntax::entity::Node::empty(
            node::entity::Meta::from("a".to_string()),
            node::entity::Variant::Attribute(node::entity::Attribute::new(expr_arena)),
        );

        let mut p = mock::entity::package("p");
        p.variant.children_mut().unwrap().push(1.into());

        let resolved = resolve(vec![p, a]);

        let a_idx = get_child(&resolved, 0.into(), "a");
        let expr_arena = resolved[a_idx].variant.expressions().next().unwrap();

        let (body_idx, domain_idx) = match &expr_arena[0.into()].variant {
            node::expression::Variant::Comprehension(c) => {
                (c.expression, node::expression::Idx::from(0))
            }
            _ => panic!("expected comprehension"),
        };

        let target = match &expr_arena[body_idx].variant {
            node::expression::Variant::Identifier(ident) => ident.target.target(),
            _ => panic!("expected identifier"),
        };
        match target {
            Some(Target::Inner(idx)) => {
                assert_eq!(*idx, domain_idx);
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

        let markup = resolved[0.into()].markups().next().cloned().unwrap();
        let rf = markup.container.references().next().cloned().unwrap();
        assert!(rf.is_valid());
        assert_eq!(resolved[*rf.target().unwrap()].meta.label, "p");
    }
}
