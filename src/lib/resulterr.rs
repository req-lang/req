use std::collections::HashMap;

pub struct ResultErr<U, T, E>(pub (U, Result<T, E>));

impl<U, T, E> ResultErr<U, T, E> {
    fn unwrap(self) -> (U, Result<T, E>) {
        let Self((key, result)) = self;
        (key, result)
    }
}

impl<T, E, U> FromIterator<ResultErr<U, T, E>> for (HashMap<U, T>, Vec<E>)
where
    U: Eq + std::hash::Hash,
{
    fn from_iter<I: IntoIterator<Item = ResultErr<U, T, E>>>(iter: I) -> (HashMap<U, T>, Vec<E>) {
        let (ok, errs): (Vec<_>, Vec<_>) = iter
            .into_iter()
            .map(ResultErr::unwrap)
            .partition(|(_, result)| result.is_ok());
        (
            ok.into_iter()
                .map(|(u, ok)| (u, ok.ok().unwrap()))
                .collect(),
            errs.into_iter()
                .map(|(_, err)| err.err().unwrap())
                .collect(),
        )
    }
}
