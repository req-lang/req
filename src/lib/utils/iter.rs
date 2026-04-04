use crate::utils::children::ChildrenIter;

pub struct NodeIter<N> {
    stack: Vec<N>,
}

impl<N> NodeIter<N> {
    pub fn new(node: N) -> Self {
        NodeIter { stack: vec![node] }
    }
}

impl<N> Iterator for NodeIter<N>
where
    N: ChildrenIter<N>,
{
    type Item = N;

    fn next(&mut self) -> Option<Self::Item> {
        let node = self.stack.pop()?;
        let children = node.children_iter().collect::<Vec<_>>().into_iter().rev();
        self.stack.extend(children);
        Some(node)
    }
}
