//! Fixed-size move-to-front (MRU) list used by the match model.

pub struct RecentList<T: Copy> {
    slots: Vec<T>,
}

impl<T: Copy> RecentList<T> {
    pub fn new(items: Vec<T>) -> Self {
        RecentList { slots: items }
    }

    /// Insert `value` at the front, shifting the rest back and dropping the
    /// oldest entry.
    pub fn push(&mut self, value: T) {
        for i in (1..self.slots.len()).rev() {
            self.slots[i] = self.slots[i - 1];
        }
        self.slots[0] = value;
    }

    /// Return the entry at `index` and move it to the front.
    pub fn promote(&mut self, index: usize) -> T {
        let value = self.slots[index];
        for i in (1..=index).rev() {
            self.slots[i] = self.slots[i - 1];
        }
        self.slots[0] = value;
        value
    }
}
