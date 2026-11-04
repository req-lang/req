use std::collections::HashMap;

use crate::{
    node::{self, identifier, path::Path, reference, tree::Tree},
    semantics::path::Location,
};

pub mod entity;
pub mod expression;

/// Resolves every reference and identifier of the tree, `paths` must come from the same entities
pub fn link(tree: &Tree, paths: &node::entity::Arena<Location>) -> Links {
    let mut links = Links {
        references: tree.references.parallel(),
        identifiers: tree.identifiers.parallel(),
    };
    entity::link(tree, paths, &mut links);
    links
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum Issue {
    #[error("Reference target does not exist")]
    NotFound,
    #[error("Reference target can points to several, diverging locations")]
    Ambiguous,
}

/// What each path points to, `None` when it does not resolve
#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct Links {
    /// Entities can only refer to other entities
    pub references: reference::Arena<Option<node::entity::Idx>>,
    /// Identifiers can also refer to builtin types, enum members and local variables
    pub identifiers: identifier::Arena<Option<Target>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    Entity(node::entity::Idx),
    Builtin(BuiltinKind),
    Enum(node::entity::Idx, usize),
    // INVARIANT: The expression is always a set element
    Local(node::expression::Idx),
}

impl Target {
    /// What a path points to when it is not a local variable
    pub fn find(ctx: &FindCtx, rf: &Path) -> Option<Self> {
        let parent = rf.parent().unwrap_or(rf.clone());
        let label = rf.label();
        if let Some(idx) = entity::find(ctx, &parent)
            && let Some(pos) = ctx.members(idx).iter().position(|s| *s == label)
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

pub struct FindCtx<'a> {
    entities: &'a node::entity::Arena<node::entity::Node>,
    nodes: &'a HashMap<&'a Path, node::entity::Idx>,
    current: &'a Path,
    imports: Vec<&'a Path>,
}

impl<'a> FindCtx<'a> {
    fn new(
        entities: &'a node::entity::Arena<node::entity::Node>,
        nodes: &'a HashMap<&'a Path, node::entity::Idx>,
        current: &'a Path,
        imports: Vec<&'a Path>,
    ) -> Self {
        Self {
            entities,
            nodes,
            current,
            imports,
        }
    }

    fn members(&self, idx: node::entity::Idx) -> &[String] {
        match &self.entities[idx].variant {
            node::entity::Variant::Type(typ) => &typ.members,
            _ => &[],
        }
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use crate::{
        mock::{
            self,
            entity::{
                add, add_parent, attribute, find, package, part, reference, requirement, typ,
            },
            expression::{binary, identifier, insert},
        },
        node::{
            self,
            entity::{Attribute, Idx, Meta, RequirementVariant, Variant},
            markup::{Item, Markup},
            tree::{Draft, Tree},
        },
        semantics::{
            link::{Links, Target, link},
            path::paths,
        },
    };

    fn resolved(draft: Draft) -> (Tree, Links) {
        let tree = Tree::from(draft);
        let links = link(&tree, &paths(&tree.entities));
        (tree, links)
    }

    fn with_restriction(draft: &mut Draft, label: &str, target: &str) -> Idx {
        let mut req = requirement(label);
        let target = reference(draft, target);
        req.variant.requirement_mut().unwrap().restriction = Some(target);
        add(draft, req)
    }

    fn add_attribute(draft: &mut Draft, label: &str, domain: &str) -> Idx {
        let attr = attribute(draft, label, domain);
        add(draft, attr)
    }

    fn assert_resolves_to(tree: &Tree, links: &Links, idx: Idx, label: &str) {
        let rf = tree.entities[idx].variant.references().next().unwrap();
        match links.references[rf] {
            Some(target) => assert_eq!(tree.entities[target].meta.label, label),
            None => panic!("expected resolved reference"),
        }
    }

    #[test]
    fn resolves_complete_path() {
        let mut draft = Draft::default();
        let x = add_attribute(&mut draft, "x", "real");
        let r1 = with_restriction(&mut draft, "r1", "p::x");
        add_parent(&mut draft, package("p"), &[x, r1]);

        let (tree, links) = resolved(draft);

        assert_resolves_to(&tree, &links, r1, "x");
    }

    #[test]
    fn resolves_parent_path() {
        let mut draft = Draft::default();
        let x = add_attribute(&mut draft, "x", "real");
        let r1 = with_restriction(&mut draft, "r1", "x");
        add_parent(&mut draft, package("p"), &[x, r1]);

        let (tree, links) = resolved(draft);

        assert_resolves_to(&tree, &links, r1, "x");
    }

    #[test]
    fn resolves_parent_subpath() {
        let mut draft = Draft::default();
        let x = add_attribute(&mut draft, "x", "real");
        let y = add_parent(&mut draft, part("y"), &[x]);
        let r1 = with_restriction(&mut draft, "r1", "y::x");
        add_parent(&mut draft, package("p"), &[y, r1]);

        let (tree, links) = resolved(draft);

        assert_resolves_to(&tree, &links, r1, "x");
    }

    #[test]
    fn resolves_import_subpath() {
        let mut draft = Draft::default();
        let x = add_attribute(&mut draft, "x", "real");
        let y = add_parent(&mut draft, part("y"), &[x]);
        let mut q = package("q");
        let import = reference(&mut draft, "p::y");
        q.variant.imports_mut().unwrap().push(import);
        let r1 = with_restriction(&mut draft, "r1", "x");
        let q = add_parent(&mut draft, q, &[r1]);
        add_parent(&mut draft, package("p"), &[y, q]);

        let (tree, links) = resolved(draft);

        let r1 = find(&tree, q, "r1");
        assert_resolves_to(&tree, &links, r1, "x");
    }

    #[test]
    fn resolves_attribute_domain_to_entity() {
        let mut draft = Draft::default();
        let y = add_attribute(&mut draft, "y", "real");
        let x = add_attribute(&mut draft, "x", "p::y");
        add_parent(&mut draft, package("p"), &[y, x]);

        let (tree, links) = resolved(draft);

        let domain = tree.entities[x].variant.expressions().next().unwrap();
        let target = match &tree.expressions[domain].variant {
            node::expression::Variant::Identifier(ident) => links.identifiers[ident.target],
            _ => panic!("expected identifier"),
        };

        match target {
            Some(Target::Entity(idx)) => assert_eq!(tree.entities[idx].meta.label, "y"),
            other => panic!("expected entity target, got {:?}", other),
        }
    }

    #[test]
    fn resolves_attribute_domain_to_builtin() {
        let mut draft = Draft::default();
        let x = add_attribute(&mut draft, "x", "real");
        add_parent(&mut draft, package("p"), &[x]);

        let (tree, links) = resolved(draft);

        let domain = tree.entities[x].variant.expressions().next().unwrap();
        let target = match &tree.expressions[domain].variant {
            node::expression::Variant::Identifier(ident) => links.identifiers[ident.target],
            _ => panic!("expected identifier"),
        };
        assert!(
            matches!(target, Some(Target::Builtin(_))),
            "expected builtin target, got {:?}",
            target,
        );
    }

    #[test]
    fn resolves_type_enum() {
        let mut draft = Draft::default();
        let t = add(&mut draft, typ("t", &["a", "b", "c"]));
        let x = add_attribute(&mut draft, "x", "t");

        let lhs = identifier(&mut draft, "x");
        let rhs = identifier(&mut draft, "t::a");
        let bin = binary(
            &mut draft,
            node::expression::BinaryOperator::Equal,
            lhs,
            rhs,
        );
        let mut r1 = requirement("r1");
        r1.variant.requirement_mut().unwrap().variant = RequirementVariant::Formal(bin);
        let r1 = add(&mut draft, r1);
        add_parent(&mut draft, package("p"), &[t, x, r1]);

        let (tree, links) = resolved(draft);

        let target = match &tree.expressions[rhs].variant {
            node::expression::Variant::Identifier(ident) => links.identifiers[ident.target],
            _ => panic!("expected identifier"),
        };
        assert_eq!(target, Some(Target::Enum(t, 0)));
    }

    #[test]
    fn resolves_comprehension_body_to_local_variable() {
        let mut draft = Draft::default();
        let domain = identifier(&mut draft, "S");
        let body = identifier(&mut draft, "x");
        let set = node::expression::SetElement::new("x".to_string(), domain);
        let comprehension = node::expression::Comprehension::new(
            Some(node::expression::Aggregator::All),
            set,
            body,
        );
        let root = insert(
            &mut draft,
            node::expression::Variant::Comprehension(comprehension),
        );
        let a = node::entity::Node::new(
            Meta::from("a".to_string()),
            Variant::Attribute(Attribute::new(root)),
        );
        let a = add(&mut draft, a);
        add_parent(&mut draft, package("p"), &[a]);

        let (tree, links) = resolved(draft);

        let target = match &tree.expressions[body].variant {
            node::expression::Variant::Identifier(ident) => links.identifiers[ident.target],
            _ => panic!("expected identifier"),
        };
        assert_eq!(target, Some(Target::Local(root)));
    }

    #[test]
    fn resolves_markup_reference() {
        let mut draft = Draft::default();
        let mut p = mock::entity::package("p");
        let rf = reference(&mut draft, "p");
        p.meta.comment = Some(Markup::from(vec![Item::Reference(rf)]));
        add(&mut draft, p);

        let (_, links) = resolved(draft);

        assert_eq!(links.references[rf], Some(0.into()));
    }
}
