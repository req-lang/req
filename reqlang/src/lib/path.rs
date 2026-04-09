use crate::{
    node::{NodeRef, entity::Label},
    reference::Reference,
    utils::children::ChildrenIter,
};

pub struct NodePathIter<N> {
    stack: Vec<(Reference, N)>,
}

impl<N: Label> NodePathIter<NodeRef<N>> {
    pub fn new(node: NodeRef<N>) -> Self {
        let label = node.borrow().label();
        NodePathIter {
            stack: vec![(Reference::new(label), node)],
        }
    }
}

impl<N> Iterator for NodePathIter<NodeRef<N>>
where
    N: ChildrenIter<NodeRef<N>> + Label,
{
    type Item = (Reference, NodeRef<N>);

    fn next(&mut self) -> Option<Self::Item> {
        let (path, node) = self.stack.pop()?;
        let children = node
            .borrow()
            .children_iter()
            .collect::<Vec<_>>()
            .into_iter()
            .rev();
        let with_paths = children.map(|child| (&path + &child.clone().borrow().label()[..], child));
        self.stack.extend(with_paths);
        Some((path, node))
    }
}
