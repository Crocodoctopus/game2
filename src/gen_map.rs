use std::marker::PhantomData;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Handle(u16, u16);

impl Handle {
    fn empty() -> Self {
        Self(0, u16::max_value())
    }

    fn inner(&self) -> Option<u16> {
        if self.1 == u16::max_value() {
            return None;
        }
        return Some(self.1);
    }
}

#[derive(Debug)]
pub struct GenMap<T> {
    id_counter: u16,
    free_handles: Vec<Handle>,
    id_to_index: Vec<u16>,

    keys: Vec<Handle>,
    values: Vec<T>,
}

impl<T> GenMap<T> {
    pub fn new() -> Self {
        Self {
            id_counter: 0,
            free_handles: vec![],
            id_to_index: vec![],

            keys: vec![],
            values: vec![],
        }
    }

    pub fn insert(&mut self, t: T) -> Handle {
        // Get an ID.
        let handle = self.free_handles.pop().unwrap_or_else(|| {
            self.id_counter += 1;
            self.id_to_index.push(u16::max_value());
            Handle(0, self.id_counter - 1)
        });

        self.id_to_index[handle.1 as usize] = self.keys.len() as u16;
        self.keys.push(handle);
        self.values.push(t);

        handle
    }

    pub fn remove(&mut self, handle: Handle) -> Option<T> {
        assert_eq!(self.keys.len(), self.values.len());
        assert_eq!(self.id_to_index.len(), self.id_counter as usize);

        // If there are no value, early return.
        if self.keys.len() == 0 {
            return None;
        }

        // If there is only one value, return it.
        if self.keys.len() == 1 {
            self.keys.pop();
            self.id_to_index[0] = u16::max_value();
            return self.values.pop();
        }

        // Find the element to be removed.
        let target_index = self.id_to_index[handle.1 as usize] as usize;
        if handle != self.keys[target_index] {
            return None;
        }

        // Pop the last elements.
        let end_key = self.keys.pop().unwrap();
        let end_value = self.values.pop().unwrap();

        // Swap out the target elements with the end elements.
        let target_key = std::mem::replace(&mut self.keys[target_index], end_key);
        let target_value = std::mem::replace(&mut self.values[target_index], end_value);

        //
        self.id_to_index[end_key.1 as usize] = target_index as u16;
        self.id_to_index[target_key.1 as usize] = u16::max_value();

        //
        self.free_handles
            .push(Handle(target_key.0.wrapping_add(1), target_key.1));
        return Some(target_value);
    }

    pub fn get(&self, handle: &Handle) -> Option<&T> {
        let index = self.id_to_index[handle.1 as usize] as usize;

        if self.keys[index].0 != handle.0 {
            return None;
        }

        return Some(&self.values[index as usize]);
    }

    pub fn get_mut(&mut self, handle: &Handle) -> Option<&mut T> {
        let index = self.id_to_index[handle.1 as usize] as usize;

        if self.keys[index].0 != handle.0 {
            return None;
        }

        return Some(&mut self.values[index as usize]);
    }

    pub fn iter(&self) -> impl Iterator<Item = (&Handle, &T)> {
        self.keys.iter().zip(self.values.iter())
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&mut Handle, &mut T)> {
        self.keys.iter_mut().zip(self.values.iter_mut())
    }
}
