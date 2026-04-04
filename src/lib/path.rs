use crate::{node::entity::Label, reference::Reference, utils::children::ChildrenIter};

pub struct NodePathIter<N> {
    stack: Vec<(Reference, N)>,
}

impl<N: Label> NodePathIter<N> {
    pub fn new(node: N) -> Self {
        NodePathIter {
            stack: vec![(Reference::new(node.label()), node)],
        }
    }
}

impl<N> Iterator for NodePathIter<N>
where
    N: ChildrenIter<N> + Label,
{
    type Item = (Reference, N);

    fn next(&mut self) -> Option<Self::Item> {
        let (path, node) = self.stack.pop()?;
        let children = node.children_iter().collect::<Vec<_>>().into_iter().rev();
        let with_paths = children.map(|child| (&path + &child.label()[..], child));
        self.stack.extend(with_paths);
        Some((path, node))
    }
}
