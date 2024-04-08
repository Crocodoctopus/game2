use std::ops::Add;

pub struct HandleMap<K, V> {
    counter: K,
    free_keys: Vec<K>,
    keys: Vec<K>,
    values: Vec<V>,
}

impl<K: Ord + Add<usize, Output = K> + Into<usize> + Default + Copy, V> HandleMap<K, V> {
    fn new() -> Self {
        Self {
            counter: <_>::default(),
            free_keys: Vec::new(),
            keys: Vec::new(),
            values: Vec::new(),
        }
    }

    fn generate_key(&mut self) -> K {
        if let Some(key) = self.free_keys.pop() {
            return key;
        }
        let tmp = self.counter;
        self.counter = self.counter + 1;
        return tmp;
    }

    pub fn insert(&mut self, value: V) -> K {
        let key = self.generate_key();
        self.keys.push(key);
        self.values.push(value);
        return key;
    }

    /*
    pub fn remove(&mut self, key: K) -> V {
        let index = keys[key as usize];
    }
    */
}
