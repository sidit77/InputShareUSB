use std::fmt::{Debug, Formatter};
use druid::Data;
use druid::im::HashSet;
use druid::widget::ListIter;
use serde::{Deserialize, Serialize};
use yawi::VirtualKey;

#[derive(Copy, Clone, Eq, PartialEq, Data, Serialize, Deserialize)]
pub struct VirtualKeySet {
    keys: [u64; 4]
}

impl VirtualKeySet {
    pub fn new() -> Self {
        Self {
            keys: [0; 4],
        }
    }

    #[inline]
    fn index(key: VirtualKey) -> (usize, u64) {
        let id = u8::from(key);
        let index = (id >> 6) as usize;
        let mask = 1u64 << (id & 0b0011_1111) as u64;
        (index, mask)
    }

    pub fn insert(&mut self, key: VirtualKey) {
        let (index, mask) = Self::index(key);
        self.keys[index] |= mask;
    }

    pub fn remove(&mut self, key: VirtualKey) {
        let (index, mask) = Self::index(key);
        self.keys[index] &= !mask;
    }

    pub fn contains(self, key: VirtualKey) -> bool {
        let (index, mask) = Self::index(key);
        self.keys[index] & mask != 0
    }

    pub fn is_superset(self, other: VirtualKeySet) -> bool {
        self.keys
            .iter()
            .zip(other.keys.iter())
            .all(|(set, sub)| {
                set & sub == *sub
            })
    }

    pub fn iter(self) -> impl Iterator<Item = VirtualKey> {
        (0..4)
            .into_iter()
            .flat_map(move |index|(0..64)
                .into_iter()
                .filter(move |i| self.keys[index] & (1 << i) != 0)
                .filter_map(move |i| VirtualKey::try_from(((index << 6) | i) as u8).ok()))
    }

    pub fn len(self) -> usize {
        self.keys
            .iter()
            .map(|i| i.count_ones() as usize)
            .sum()
    }

}

impl Debug for VirtualKeySet {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_set().entries(self.iter()).finish()
    }
}

impl FromIterator<VirtualKey> for VirtualKeySet {
    fn from_iter<T: IntoIterator<Item=VirtualKey>>(iter: T) -> Self {
        let mut result = VirtualKeySet::new();
        for key in iter {
            result.insert(key);
        }
        result
    }
}

impl From<&HashSet<VirtualKey>> for VirtualKeySet {
    fn from(value: &HashSet<VirtualKey>) -> Self {
        let mut result = VirtualKeySet::new();
        for key in value {
            result.insert(*key);
        }
        result
    }
}

impl ListIter<VirtualKey> for VirtualKeySet {
    fn for_each(&self, mut cb: impl FnMut(&VirtualKey, usize)) {
        for (i, item) in self.iter().enumerate() {
            cb(&item, i);
        }
    }

    fn for_each_mut(&mut self, mut cb: impl FnMut(&mut VirtualKey, usize)) {
        let mut updated = VirtualKeySet::new();
        for (i, mut item) in self.iter().enumerate() {
            cb(&mut item, i);
            updated.insert(item);
        }
        *self = updated;
    }

    fn data_len(&self) -> usize {
        self.len()
    }
}

impl ListIter<(VirtualKeySet, VirtualKey)> for VirtualKeySet {
    fn for_each(&self, mut cb: impl FnMut(&(VirtualKeySet, VirtualKey), usize)) {
        for (i, item) in self.iter().enumerate() {
            cb(&(*self, item), i);
        }
    }

    fn for_each_mut(&mut self, mut cb: impl FnMut(&mut (VirtualKeySet, VirtualKey), usize)) {
        let mut updated = *self;
        for (i, mut item) in self.iter().enumerate() {
            let mut tuple = (updated, item);
            cb(&mut tuple, i);
            updated = tuple.0;
            //updated.remove(item);
            //updated.insert(tuple.1);
        }
        *self = updated;
    }

    fn data_len(&self) -> usize {
        self.len()
    }
}