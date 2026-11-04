use crate::{
    node::{entity::Arena, tree::Tree},
    semantics::{connection::entity::Neighbors, link::Links},
};

pub mod entity;
pub mod graph;
pub mod issue;

pub fn neighbors(tree: &Tree, links: &Links) -> Arena<Neighbors> {
    let mut neighbors = tree.entities.parallel();
    entity::neighbors(tree, links, &mut neighbors);
    neighbors
}

pub fn diagnostics(tree: &Tree, neighbors: &mut Arena<Neighbors>) {
    entity::diagnostics(tree, neighbors)
}

#[cfg(test)]
mod tests {
    use crate::{
        mock::entity::{add, add_parent, package, reference, requirement},
        node::{
            entity::{TraceabilityKind, TraceabilityLink},
            tree::{Draft, Tree},
        },
        semantics::{
            connection::{diagnostics, issue::IssueKind, neighbors},
            link::link,
            path::paths,
        },
    };

    #[test]
    fn requirement_tracing_itself_is_a_cycle() {
        let mut draft = Draft::default();
        let mut r = requirement("r");
        let target = reference(&mut draft, "p::r");
        r.variant
            .requirement_mut()
            .unwrap()
            .traceability
            .push(TraceabilityLink::new(TraceabilityKind::Refinement, target));
        let r = add(&mut draft, r);
        add_parent(&mut draft, package("p"), &[r]);

        let tree = Tree::from(draft);
        let links = link(&tree, &paths(&tree.entities));
        let mut neighbors = neighbors(&tree, &links);
        diagnostics(&tree, &mut neighbors);

        assert!(
            neighbors[r]
                .inbound
                .traceability
                .issues
                .contains(&IssueKind::CyclicTrace)
        );
    }

    #[test]
    fn restriction_issue_is_on_the_restriction() {
        let mut draft = Draft::default();
        let other = add(&mut draft, package("other"));
        let mut r = requirement("r");
        // Restricting a package instead of a part or an attribute
        let on = reference(&mut draft, "p::other");
        r.variant.requirement_mut().unwrap().restriction = Some(on);
        let r = add(&mut draft, r);
        add_parent(&mut draft, package("p"), &[other, r]);

        let tree = Tree::from(draft);
        let links = link(&tree, &paths(&tree.entities));
        let mut neighbors = neighbors(&tree, &links);
        diagnostics(&tree, &mut neighbors);

        let restriction = neighbors[r].outbound.restriction.as_ref().unwrap();
        assert_eq!(restriction.target, other);
        assert!(restriction.issues.contains(&IssueKind::NotAPartOrAttribute));
    }

    #[test]
    fn requirement_without_upward_traceability_is_flagged_upward() {
        let mut draft = Draft::default();
        let r = add(&mut draft, requirement("r"));
        add_parent(&mut draft, package("p"), &[r]);

        let tree = Tree::from(draft);
        let links = link(&tree, &paths(&tree.entities));
        let mut neighbors = neighbors(&tree, &links);
        diagnostics(&tree, &mut neighbors);

        assert!(
            neighbors[r]
                .outbound
                .traceability
                .issues
                .contains(&IssueKind::MissingTrace)
        );
    }
}
