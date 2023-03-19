use yawi::VirtualKey;
use crate::hook::util::VirtualKeySet;


mod util {
    use std::fmt::{Debug, Formatter};
    use druid::im::HashSet;
    use yawi::VirtualKey;

    #[derive(Copy, Clone, Eq, PartialEq)]
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

        pub fn iter(self) -> impl Iterator<Item = VirtualKey> {
            (0..4)
                .into_iter()
                .flat_map(move |index|(0..64)
                    .into_iter()
                    .filter(move |i| self.keys[index] & (1 << i) != 0)
                    .filter_map(move |i| VirtualKey::try_from(((index << 6) | i) as u8).ok()))
        }

    }

    impl Debug for VirtualKeySet {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            f.debug_set().entries(self.iter()).finish()
        }
    }

    impl From<HashSet<VirtualKey>> for VirtualKeySet {
        fn from(value: HashSet<VirtualKey>) -> Self {
            let mut result = VirtualKeySet::new();
            for key in value {
                result.insert(key);
            }
            result
        }
    }

}
