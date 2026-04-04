pub trait Children<N>
where
    Self: Sized,
{
    fn children(&self) -> Option<&Vec<N>>;
}

pub trait ChildrenMut<N>
where
    Self: Sized,
{
    fn children_mut(&mut self) -> Option<&mut Vec<N>>;
}

pub trait IntoChildren<N>
where
    Self: Sized,
{
    fn into_children(self) -> Option<Vec<N>>;
}

pub trait ChildrenIter<N> {
    fn children_iter(&self) -> impl Iterator<Item = N>;
}
