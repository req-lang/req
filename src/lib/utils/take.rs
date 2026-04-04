use std::collections::HashMap;
use std::hash::Hash;

pub trait Take<K, V> {
    fn take(&mut self, key: &K) -> V;
}

impl<K: Hash + Eq, V: Default> Take<K, V> for HashMap<K, V> {
    fn take(&mut self, key: &K) -> V {
        self.remove(key).unwrap_or_default()
    }
}
