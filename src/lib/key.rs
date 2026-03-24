use std::hash::{Hash, Hasher};

#[derive(Debug)]
pub(crate) struct NodeKey<'n, N>(&'n N);

impl<'n, N> Copy for NodeKey<'n, N> {}

impl<'n, N> Clone for NodeKey<'n, N> {
    fn clone(&self) -> Self {
        *self
    }
}

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

impl<'n, N> From<&'n N> for NodeKey<'n, N> {
    fn from(value: &'n N) -> Self {
        Self(&value)
    }
}

impl<N> Eq for NodeKey<'_, N> {}
