use std::{cell::RefCell, rc::Weak};

use crate::{
    node::{self, NodeRef},
    reference::Reference,
    semantics::link::entity::{Ctx, Node, Paths, build_paths, init_nodes, link},
    syntax,
};

pub mod convert;
pub mod entity;
pub mod expression;
pub mod markup;

pub type WeakNodeRef<T> = Weak<RefCell<T>>;
pub type Result<N> = std::result::Result<N, Reference>;
pub type ResultRef<N> = Result<WeakNodeRef<N>>;
pub type ResultTgt<N, E> = Result<expression::Target<WeakNodeRef<N>, WeakNodeRef<E>>>;

pub fn resolve(syntax: NodeRef<syntax::entity::Node>) -> NodeRef<Node> {
    let node = node::map(syntax, init_nodes);

    let root = Reference::default();
    let mut paths = Paths::default();

    build_paths(node.clone(), &root, &mut paths);
    link(&paths, node.clone(), Ctx::default());

    node
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use pretty_assertions::assert_eq;

    use crate::{
        mock,
        node::{self, NodeRef, entity as entity_node, expression as expr_node, markup},
        reference::Reference,
        semantics::link::{entity::Node, expression, resolve},
        syntax::{self, parser},
        utils::children::{ChildrenIter, ChildrenMut},
    };

    fn get_child(parent: &NodeRef<Node>, label: &str) -> NodeRef<Node> {
        parent
            .borrow()
            .variant
            .children_iter()
            .find(|n| n.borrow().variant.meta().label == label)
            .unwrap()
    }

    fn assert_resolves_to(node: &NodeRef<Node>, label: &str) {
        let rf = node.borrow().variant.references().next().cloned().unwrap();
        assert!(rf.is_ok());
        assert_eq!(
            rf.unwrap().upgrade().unwrap().borrow().variant.meta().label,
            label
        );
    }

    #[test]
    fn resolves_complete_path() {
        let p = mock::entity::package("p");
        let r1 = mock::entity::requirement("r1");
        r1.borrow_mut()
            .variant
            .requirement_mut()
            .unwrap()
            .restriction = Some(Reference::new("p::x".to_string()));
        {
            let mut p_mut = p.borrow_mut();
            let children = p_mut.variant.children_mut().unwrap();
            children.push(mock::entity::attribute("x", "real"));
            children.push(r1);
        }

        let resolved = resolve(p);

        assert_resolves_to(&get_child(&resolved, "r1"), "x");
    }

    #[test]
    fn resolves_parent_path() {
        let p = mock::entity::package("p");
        let r1 = mock::entity::requirement("r1");
        r1.borrow_mut()
            .variant
            .requirement_mut()
            .unwrap()
            .restriction = Some(Reference::new("x".to_string()));
        {
            let mut p_mut = p.borrow_mut();
            let children = p_mut.variant.children_mut().unwrap();
            children.push(mock::entity::attribute("x", "real"));
            children.push(r1);
        }

        let resolved = resolve(p);

        assert_resolves_to(&get_child(&resolved, "r1"), "x");
    }

    #[test]
    fn resolves_parent_subpath() {
        let p = mock::entity::package("p");
        let y = mock::entity::part("y");
        let r1 = mock::entity::requirement("r1");
        r1.borrow_mut()
            .variant
            .requirement_mut()
            .unwrap()
            .restriction = Some(Reference::new("y::x".to_string()));
        y.borrow_mut()
            .variant
            .children_mut()
            .unwrap()
            .push(mock::entity::attribute("x", "real"));
        {
            let mut p_mut = p.borrow_mut();
            let children = p_mut.variant.children_mut().unwrap();
            children.push(y);
            children.push(r1);
        }

        let resolved = resolve(p);

        assert_resolves_to(&get_child(&resolved, "r1"), "x");
    }

    #[test]
    fn resolves_import_subpath() {
        let p = mock::entity::package("p");
        let y = mock::entity::part("y");
        let q = mock::entity::package("q");
        let r1 = mock::entity::requirement("r1");
        r1.borrow_mut()
            .variant
            .requirement_mut()
            .unwrap()
            .restriction = Some(Reference::new("x".to_string()));
        q.borrow_mut()
            .variant
            .imports_mut()
            .unwrap()
            .push(Reference::new("p::y".to_string()));
        y.borrow_mut()
            .variant
            .children_mut()
            .unwrap()
            .push(mock::entity::attribute("x", "real"));
        q.borrow_mut().variant.children_mut().unwrap().push(r1);
        {
            let mut p_mut = p.borrow_mut();
            let children = p_mut.variant.children_mut().unwrap();
            children.push(y); // y must precede q so its path is set before import resolution
            children.push(q);
        }

        let resolved = resolve(p);

        assert_resolves_to(&get_child(&get_child(&resolved, "q"), "r1"), "x");
    }

    fn get_expr_from_attr(attr: &NodeRef<Node>) -> NodeRef<expression::Node> {
        attr.borrow().variant.expressions().next().cloned().unwrap()
    }

    #[test]
    fn resolves_attribute_domain_to_entity() {
        let p = mock::entity::package("p");
        let y = mock::entity::attribute("y", "real");
        let x = mock::entity::attribute("x", "p::y");
        {
            let mut p_mut = p.borrow_mut();
            let children = p_mut.variant.children_mut().unwrap();
            children.push(y);
            children.push(x);
        }

        let resolved = resolve(p);

        let x = get_child(&resolved, "x");
        let domain = get_expr_from_attr(&x);
        let borrow = domain.borrow();
        let target = match &borrow.variant {
            expr_node::Variant::Identifier(ident) => ident.target.as_ref().unwrap(),
            _ => panic!("expected identifier"),
        };
        match target {
            expression::Target::Entity(weak) => {
                assert_eq!(weak.upgrade().unwrap().borrow().variant.meta().label, "y");
            }
            other => panic!("expected entity target, got {:?}", other),
        }
    }

    #[test]
    fn resolves_attribute_domain_to_builtin() {
        let p = mock::entity::package("p");
        p.borrow_mut()
            .variant
            .children_mut()
            .unwrap()
            .push(mock::entity::attribute("x", "real"));

        let resolved = resolve(p);

        let x = get_child(&resolved, "x");
        let domain = get_expr_from_attr(&x);
        let borrow = domain.borrow();
        let target = match &borrow.variant {
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
        let p = mock::entity::package("p");

        // forall x in S, x  — "x" in body should resolve to local var (domain), not entity
        let domain = mock::expression::identifier("S");
        let body = mock::expression::identifier("x");
        let set = expr_node::SetElement::new("x".to_string(), domain);
        let forall = expr_node::Forall::new(set, body);
        let forall_node = node::new(syntax::expression::Node::new(expr_node::Variant::Forall(
            forall,
        )));

        let mut a_variant =
            entity_node::Variant::Attribute(entity_node::Attribute::new(forall_node));
        a_variant.meta_mut().label = "a".to_string();
        let a = node::new(syntax::entity::Node::new(a_variant));

        p.borrow_mut().variant.children_mut().unwrap().push(a);

        let resolved = resolve(p);

        let a = get_child(&resolved, "a");
        let forall_expr = get_expr_from_attr(&a);

        let (body_expr, domain_expr) = {
            let borrow = forall_expr.borrow();
            match &borrow.variant {
                expr_node::Variant::Forall(forall) => {
                    (forall.expression.clone(), forall.set.domain.clone())
                }
                _ => panic!("expected forall"),
            }
        };

        let borrow = body_expr.borrow();
        let target = match &borrow.variant {
            expr_node::Variant::Identifier(ident) => ident.target.as_ref().unwrap(),
            _ => panic!("expected identifier"),
        };
        match target {
            expression::Target::Expression(_, weak) => {
                assert!(Rc::ptr_eq(&weak.upgrade().unwrap(), &domain_expr));
            }
            other => panic!("expected local variable target, got {:?}", other),
        }
    }

    #[test]
    fn resolves_markup_reference() {
        let p = mock::entity::package("p");
        {
            let mut comment = markup::Container::default();
            comment
                .items
                .push(markup::Item::Reference(Reference::new("p".to_string())));
            p.borrow_mut().variant.meta_mut().comment =
                Some(parser::markup::Node { container: comment });
        }

        let resolved = resolve(p);

        let markup = resolved.borrow().variant.markups().next().cloned().unwrap();
        let rf = markup.container.references().next().cloned().unwrap();
        assert!(rf.is_ok());
        assert_eq!(
            rf.unwrap().upgrade().unwrap().borrow().variant.meta().label,
            "p"
        );
    }
}
