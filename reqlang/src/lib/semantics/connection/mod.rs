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

#[cfg(test)]
mod tests {
    use crate::{
        mock::entity::{add, add_parent, package, reference, requirement},
        node::{
            entity::{TraceabilityKind, TraceabilityLink},
            tree::Tree,
        },
        semantics::{
            connection::{issue, neighbors},
            link::link,
            path::paths,
        },
    };

    #[test]
    fn requirement_tracing_itself_is_a_cycle() {
        let mut draft = Tree::default();
        let mut r = requirement(&mut draft, "r");
        let target = reference(&mut draft, "p::r");
        r.variant
            .requirement_mut()
            .unwrap()
            .traceability
            .push(TraceabilityLink::new(TraceabilityKind::Refinement, target));
        let r = add(&mut draft, r);
        add_parent(&mut draft, package("p"), &[r]);

        let tree = draft;
        let links = link(&tree, &paths(&tree.entities));
        let neighbors = neighbors(&tree, &links);
        let issues = issue::reference::diagnostics(&tree, &links, &neighbors);

        // The link of the requirement to itself is the one in a cycle
        assert!(issues[target].contains(&issue::reference::Issue::CyclicTrace));
    }

    #[test]
    fn requirement_without_traceability_is_flagged_both_ways() {
        let mut draft = Tree::default();
        let r = requirement(&mut draft, "r");
        let r = add(&mut draft, r);
        add_parent(&mut draft, package("p"), &[r]);

        let tree = draft;
        let links = link(&tree, &paths(&tree.entities));
        let neighbors = neighbors(&tree, &links);
        let issues = issue::entity::diagnostics(&tree, &neighbors);

        assert!(issues[r].contains(&issue::entity::Issue::MissingUpwardTrace));
        assert!(issues[r].contains(&issue::entity::Issue::MissingDownwardTrace));
    }

    #[test]
    fn inhibited_requirements_are_flagged_on_the_tracing_link() {
        let mut draft = Tree::default();
        let mut parent = requirement(&mut draft, "parent");
        parent.variant.requirement_mut().unwrap().inhibitor.bottom = true;
        let parent = add(&mut draft, parent);

        let mut child = requirement(&mut draft, "child");
        let link_to_parent = reference(&mut draft, "p::parent");
        let req = child.variant.requirement_mut().unwrap();
        req.inhibitor.top = true;
        req.traceability.push(TraceabilityLink::new(
            TraceabilityKind::Refinement,
            link_to_parent,
        ));
        let child = add(&mut draft, child);
        add_parent(&mut draft, package("p"), &[parent, child]);

        let tree = draft;
        let links = link(&tree, &paths(&tree.entities));
        let neighbors = neighbors(&tree, &links);
        let issues = issue::reference::diagnostics(&tree, &links, &neighbors);

        let on_link = &issues[link_to_parent];
        assert!(on_link.contains(&issue::reference::Issue::ForbiddenUpwardTrace));
        assert!(on_link.contains(&issue::reference::Issue::ForbiddenDownwardTrace));
        assert_eq!(neighbors[child].outbound.traceability.refinements, [parent]);
    }

    #[test]
    fn restriction_issue_is_on_the_restriction() {
        let mut draft = Tree::default();
        let other = add(&mut draft, package("other"));
        let mut r = requirement(&mut draft, "r");
        // Restricting a package instead of a part or an attribute
        let on = reference(&mut draft, "p::other");
        r.variant.requirement_mut().unwrap().restriction = Some(on);
        let r = add(&mut draft, r);
        add_parent(&mut draft, package("p"), &[other, r]);

        let tree = draft;
        let links = link(&tree, &paths(&tree.entities));
        let neighbors = neighbors(&tree, &links);
        let issues = issue::reference::diagnostics(&tree, &links, &neighbors);

        assert_eq!(neighbors[r].outbound.restriction, Some(other));
        assert!(issues[on].contains(&issue::reference::Issue::NotAPartOrAttribute));
    }

    #[test]
    fn requirement_without_restriction_is_flagged() {
        let mut draft = Tree::default();
        let r = requirement(&mut draft, "r");
        let r = add(&mut draft, r);
        add_parent(&mut draft, package("p"), &[r]);

        let tree = draft;
        let links = link(&tree, &paths(&tree.entities));
        let neighbors = neighbors(&tree, &links);
        let issues = issue::entity::diagnostics(&tree, &neighbors);

        assert!(issues[r].contains(&issue::entity::Issue::MissingRequirementBinding));
    }
}
