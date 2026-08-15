use slotmap::new_key_type;
use std::ops::{BitAnd, Not};

pub(crate) type EntityStorageId = u32;

new_key_type! { pub struct Entity; }

#[derive(Clone, Copy, PartialEq, PartialOrd)]
pub(crate) struct EntityPointer {
    pub archetype: u32,
    pub index: u32,
}

pub(crate) struct __Internals {}
impl __Internals {
    pub const INITIAL_COMPONENT_STORAGE_CAPACITY: u32 = 64;
    pub const COMPONENT_BITSET_WORDS: u32 = 4;
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Eq, Ord)]
pub struct ComponentMask {
    pub words: [u64; __Internals::COMPONENT_BITSET_WORDS as usize],
}
impl ComponentMask {
    pub const fn new() -> Self {
        ComponentMask {
            words: [0; __Internals::COMPONENT_BITSET_WORDS as usize],
        }
    }

    pub const fn single(uid: usize) -> Self {
        // Compile-time check to prevent out-of-bounds panics
        let max_components = __Internals::COMPONENT_BITSET_WORDS as usize * 64;
        assert!(
            uid < max_components,
            "Component UID exceeds maximum bitset capacity!"
        );

        let mut mask = Self {
            words: [0; __Internals::COMPONENT_BITSET_WORDS as usize],
        };

        // FIXED: Explicitly use 1u64 to match the array element type
        mask.words[uid / 64] |= 1u64 << (uid % 64);
        mask
    }

    pub const fn union(self, other: &Self) -> Self {
        let mut mask = Self {
            words: [0; __Internals::COMPONENT_BITSET_WORDS as usize],
        };
        let mut i = 0;

        while i < __Internals::COMPONENT_BITSET_WORDS as usize {
            mask.words[i] = self.words[i] | other.words[i];
            i += 1;
        }
        mask
    }

    pub const fn contains(&self, other: &Self) -> bool {
        let mut i = 0;

        while i < __Internals::COMPONENT_BITSET_WORDS as usize {
            if (self.words[i] & other.words[i]) != other.words[i] {
                return false;
            }
            i += 1;
        }

        true
    }
}
impl BitAnd for ComponentMask {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        let mut result = [0u64; __Internals::COMPONENT_BITSET_WORDS as usize];
        for i in 0..__Internals::COMPONENT_BITSET_WORDS as usize {
            result[i] = self.words[i] & rhs.words[i];
        }
        ComponentMask { words: result }
    }
}
impl BitAnd for &ComponentMask {
    type Output = ComponentMask;

    fn bitand(self, rhs: Self) -> Self::Output {
        let mut result = [0u64; __Internals::COMPONENT_BITSET_WORDS as usize];
        for i in 0..__Internals::COMPONENT_BITSET_WORDS as usize {
            result[i] = self.words[i] & rhs.words[i];
        }
        ComponentMask { words: result }
    }
}
impl Not for ComponentMask {
    type Output = Self;

    fn not(self) -> Self::Output {
        let mut result = [0u64; __Internals::COMPONENT_BITSET_WORDS as usize];
        for i in 0..__Internals::COMPONENT_BITSET_WORDS as usize {
            result[i] = !self.words[i];
        }
        ComponentMask { words: result }
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord)]
pub struct ComponentMetadata {
    pub uid: usize,
    pub mask: ComponentMask,
    pub size: usize,
    pub alignment: usize,
}

pub trait Component: 'static {
    const METADATA: ComponentMetadata;
}

#[macro_export]
macro_rules! register_components {
    ($($t:ty),* $(,)?) => {
        $crate::register_components!(@impl 0, $($t),*);
    };

    (@impl $current_id:expr, $(,)?) => {};

    (@impl $current_id:expr, $head:ty $(, $tail:ty)* $(,)?) => {
        impl $crate::ecs::common::Component for $head {
            // FIXED: Uses ComponentMetadata type and struct initialization
            const METADATA: $crate::ecs::common::ComponentMetadata = $crate::ecs::common::ComponentMetadata {
                uid: $current_id,
                // FIXED: Uses ComponentMask::single
                mask: $crate::ecs::common::ComponentMask::single($current_id),
                size: std::mem::size_of::<Self>(),
                alignment: std::mem::align_of::<Self>(),
            };
        }
        $crate::register_components!(@impl ($current_id + 1), $($tail),*);
    };
}
