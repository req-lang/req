use std::hash::{Hash, Hasher};

#[derive(Debug, Copy, Clone)]
pub(crate) struct NodeKey<'a, N>(pub &'a N);

impl<N> Hash for NodeKey<'_, N> {
    fn hash<H: Hasher>(&self, s: &mut H) {
        (self.0 as *const N as usize).hash(s);
    }
}

impl<N> PartialEq for NodeKey<'_, N> {
    fn eq(&self, o: &Self) -> bool {
        std::ptr::eq(self.0, o.0)
    }
}

impl<N> From<&N> for NodeKey<'_, N> {
    fn from(value: &N) -> Self {
        Self(&value)
    }
}

impl<N> Eq for NodeKey<'_, N> {}
