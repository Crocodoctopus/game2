use std::collections::HashMap;



pub struct IndexBiMap<K, V: Into<usize> + From<usize> + Copy> {
    keys: HashMap<K, V>,
    values: Vec<V>,
}

impl<K, V> IndexBiMap where V: Into<usize> + From<usize> + Copy {
    fn new() -> Self {
        Self {
            keys: HashMap::new(),
            values: Vec::new(),
        }
    }

    pub fn insert(&mut self, key: Key, value: V) {
        self.insert(key, value);
        self.values.push(value);
    }
}
