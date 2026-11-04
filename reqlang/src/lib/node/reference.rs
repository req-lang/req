use crate::node::arena;

#[derive(Default, Debug, Copy, Clone, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub struct IdxMarker;
pub type Idx = arena::idx::Idx<IdxMarker>;
/// Paths written in entities (imports, restrictions, allocations, traceability and markup),
/// a node only refers to them by their index
pub type Arena<T> = arena::Arena<T, IdxMarker>;
