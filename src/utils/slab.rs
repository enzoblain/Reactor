use std::mem::MaybeUninit;

pub(crate) struct Slab<T> {
    items: Vec<MaybeUninit<T>>,
    free: Vec<usize>,
}

impl<T> Slab<T> {
    pub(crate) fn new(size: usize) -> Self {
        let items = (0..size).map(|_| MaybeUninit::<T>::uninit()).collect();
        let free = (0..size).collect();

        Self { items, free }
    }

    pub(crate) fn remove(&mut self, index: usize) -> T {
        let item = unsafe { self.items[index].assume_init_read() };

        self.items[index] = MaybeUninit::uninit();
        self.free.push(index);

        item
    }

    pub(crate) fn insert(&mut self, item: T) -> usize {
        let index = if let Some(i) = self.free.pop() {
            i
        } else {
            let len = self.items.len();
            let new_len = if len == 0 { 1 } else { 2 * len };

            self.items
                .extend((len..new_len).map(|_| MaybeUninit::<T>::uninit()));
            self.free.extend((len + 1)..new_len);

            len
        };

        self.items[index] = MaybeUninit::new(item);

        index
    }
}

impl<T> Drop for Slab<T> {
    fn drop(&mut self) {
        let mut is_free = vec![false; self.items.len()];
        for &i in &self.free {
            if i < is_free.len() {
                is_free[i] = true;
            }
        }

        for (slot, &free) in self.items.iter_mut().zip(is_free.iter()) {
            if !free {
                unsafe {
                    slot.assume_init_drop();
                }
            }
        }
    }
}
