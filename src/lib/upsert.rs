use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
};

pub trait Upsert<K, V> {
    fn upsert(&mut self, key: K, value: V);
}

impl<K: Eq + Hash, V> Upsert<K, V> for HashMap<K, Vec<V>> {
    fn upsert(&mut self, key: K, value: V) {
        match self.get_mut(&key) {
            Some(values) => {
                values.push(value);
            }
            None => {
                self.insert(key, vec![value]);
            }
        }
    }
}

impl<K: Eq + Hash, V: Eq + Hash> Upsert<K, V> for HashMap<K, HashSet<V>> {
    fn upsert(&mut self, key: K, value: V) {
        match self.get_mut(&key) {
            Some(values) => {
                values.insert(value);
            }
            None => {
                let mut set = HashSet::new();
                set.insert(value);
                self.insert(key, set);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{HashMap, HashSet};

    use super::Upsert;

    #[test]
    fn vec_upsert_creates_entry_on_new_key() {
        let mut map: HashMap<&str, Vec<i32>> = HashMap::new();
        map.upsert("a", 1);
        assert_eq!(map["a"], vec![1]);
    }

    #[test]
    fn vec_upsert_appends_on_existing_key() {
        let mut map: HashMap<&str, Vec<i32>> = HashMap::new();
        map.upsert("a", 1);
        map.upsert("a", 2);
        map.upsert("a", 3);
        assert_eq!(map["a"], vec![1, 2, 3]);
    }

    #[test]
    fn vec_upsert_preserves_insertion_order() {
        let mut map: HashMap<i32, Vec<i32>> = HashMap::new();
        for v in [3, 1, 2] {
            map.upsert(0, v);
        }
        assert_eq!(map[&0], vec![3, 1, 2]);
    }

    #[test]
    fn vec_upsert_handles_multiple_keys_independently() {
        let mut map: HashMap<&str, Vec<i32>> = HashMap::new();
        map.upsert("a", 1);
        map.upsert("b", 2);
        map.upsert("a", 3);
        assert_eq!(map["a"], vec![1, 3]);
        assert_eq!(map["b"], vec![2]);
    }

    #[test]
    fn set_upsert_creates_entry_on_new_key() {
        let mut map: HashMap<&str, HashSet<i32>> = HashMap::new();
        map.upsert("a", 1);
        assert_eq!(map["a"], HashSet::from([1]));
    }

    #[test]
    fn set_upsert_adds_on_existing_key() {
        let mut map: HashMap<&str, HashSet<i32>> = HashMap::new();
        map.upsert("a", 1);
        map.upsert("a", 2);
        assert_eq!(map["a"], HashSet::from([1, 2]));
    }

    #[test]
    fn set_upsert_deduplicates_values() {
        let mut map: HashMap<&str, HashSet<i32>> = HashMap::new();
        map.upsert("a", 1);
        map.upsert("a", 1);
        assert_eq!(map["a"].len(), 1);
    }

    #[test]
    fn set_upsert_handles_multiple_keys_independently() {
        let mut map: HashMap<&str, HashSet<i32>> = HashMap::new();
        map.upsert("a", 1);
        map.upsert("b", 2);
        map.upsert("a", 3);
        assert_eq!(map["a"], HashSet::from([1, 3]));
        assert_eq!(map["b"], HashSet::from([2]));
    }
}
