use crate::node::arena;

#[derive(Default, Debug, Copy, Clone, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub struct IdxMarker;
pub type Idx = arena::idx::Idx<IdxMarker>;
/// Paths written in expressions, an identifier only refers to them by their index
pub type Arena<T> = arena::Arena<T, IdxMarker>;
