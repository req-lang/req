use crate::{node::NodeRef, utils::children::ChildrenIter};

pub struct NodeIter<N> {
    stack: Vec<N>,
}

impl<N> NodeIter<N> {
    pub fn new(node: N) -> Self {
        NodeIter { stack: vec![node] }
    }
}

impl<N> Iterator for NodeIter<NodeRef<N>>
where
    N: ChildrenIter<NodeRef<N>>,
{
    type Item = NodeRef<N>;

    fn next(&mut self) -> Option<Self::Item> {
        let node = self.stack.pop()?;
        let children = node
            .borrow()
            .children_iter()
            .collect::<Vec<_>>()
            .into_iter()
            .rev();
        self.stack.extend(children);
        Some(node)
    }
}
