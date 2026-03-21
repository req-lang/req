use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
};

use crate::upsert::Upsert;

pub trait Inverse<T> {
    fn inversed(&self) -> T;
}

impl<T: Eq + Hash + Copy> Inverse<HashMap<T, HashSet<T>>> for HashMap<T, T> {
    fn inversed(&self) -> HashMap<T, HashSet<T>> {
        let mut iv = HashMap::new();
        for (k, v) in self {
            iv.upsert(*v, *k);
        }
        iv
    }
}

impl<T: Eq + Hash + Copy> Inverse<HashMap<T, HashSet<T>>> for HashMap<T, HashSet<T>> {
    fn inversed(&self) -> HashMap<T, HashSet<T>> {
        let mut iv = HashMap::new();
        for (k, v) in self {
            for x in v {
                iv.upsert(*x, *k);
            }
        }
        iv
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{HashMap, HashSet};

    use super::Inverse;

    #[test]
    fn one_to_one_inverse() {
        let map = HashMap::from([(1, 2), (3, 4)]);
        let inv = map.inversed();
        assert_eq!(inv[&2], HashSet::from([1]));
        assert_eq!(inv[&4], HashSet::from([3]));
    }

    #[test]
    fn many_to_one_inverse_merges_keys() {
        // multiple keys map to the same value → inverse collects them in a set
        let map = HashMap::from([(1, 10), (2, 10), (3, 10)]);
        let inv = map.inversed();
        assert_eq!(inv[&10], HashSet::from([1, 2, 3]));
    }

    #[test]
    fn empty_map_inverts_to_empty() {
        let map: HashMap<i32, i32> = HashMap::new();
        let inv = map.inversed();
        assert!(inv.is_empty());
    }

    #[test]
    fn set_map_empty_inverts_to_empty() {
        let map: HashMap<i32, HashSet<i32>> = HashMap::new();
        let inv = map.inversed();
        assert!(inv.is_empty());
    }

    #[test]
    fn set_map_one_to_many_inverse() {
        // one key maps to multiple values → each value maps back to the key
        let map = HashMap::from([(1, HashSet::from([10, 20, 30]))]);
        let inv = map.inversed();
        assert_eq!(inv[&10], HashSet::from([1]));
        assert_eq!(inv[&20], HashSet::from([1]));
        assert_eq!(inv[&30], HashSet::from([1]));
    }

    #[test]
    fn set_map_many_to_many_inverse() {
        let map = HashMap::from([
            (1, HashSet::from([10, 20])),
            (2, HashSet::from([20, 30])),
        ]);
        let inv = map.inversed();
        assert_eq!(inv[&10], HashSet::from([1]));
        assert_eq!(inv[&20], HashSet::from([1, 2]));
        assert_eq!(inv[&30], HashSet::from([2]));
    }
}
