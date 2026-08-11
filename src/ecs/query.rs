use std::marker::PhantomData;

use crate::ecs::{
    Entity, World,
    common::{Component, ComponentMask, ComponentMetadata, EntityPointer, EntityStorageId},
    component_storage::ComponentStorage,
};

pub trait ComponentSet {
    const LEN: usize;
    const MASK: ComponentMask;

    const COMPONENTS: &'static [&'static ComponentMetadata];

    type Fetch<'a>;

    unsafe fn fetch_from_pointers<'a>(
        ptrs_storage: *mut Vec<ComponentStorage>,
        indices: &[usize],
        components_position: EntityStorageId,
    ) -> Self::Fetch<'a>;
}

macro_rules! replace_expr {
    ($_t:ident $sub:expr) => {
        $sub
    };
}

macro_rules! impl_component_set_tuples {
    ($first:ident) => {
        impl<$first: Component> ComponentSet for ($first,) {
            const LEN: usize = 1;
            const MASK: ComponentMask = $first::METADATA.mask;

            // Compile-time slice for single-element tuple
            const COMPONENTS: &'static [&'static ComponentMetadata] = &[
                &$first::METADATA
            ];

            type Fetch<'a> = &'a $first;

            unsafe fn fetch_from_pointers<'a>(
                ptrs_storage: *mut Vec<ComponentStorage>,
                indices: &[usize],
                components_position: EntityStorageId
            ) -> Self::Fetch<'a> {
                unsafe {
                    let storage = &*ptrs_storage;
                    (storage[0]).get_unchecked::<$first>(components_position)
                }
            }
        }
    };

    ($first:ident, $( $rest:ident ),+) => {
        impl<$first: Component, $( $rest: Component ),+> ComponentSet for ($first, $( $rest, )+) {
            const LEN: usize = 1 $( + replace_expr!($rest 1) )+;
            const MASK: ComponentMask = $first::METADATA.mask $( .union(&$rest::METADATA.mask) )+;

            // Compile-time slice for multi-element tuples
            const COMPONENTS: &'static [&'static ComponentMetadata] = &[
                &$first::METADATA,
                $( &$rest::METADATA ),+
            ];

            type Fetch<'a> = (&'a $first, $(&'a $rest,)+);

            unsafe fn fetch_from_pointers<'a>(
                ptrs_storage: *mut Vec<ComponentStorage>,
                indices: &[usize],
                components_position: EntityStorageId,
            ) -> Self::Fetch<'a> {
                unsafe {
                    let storage = &*ptrs_storage;
                    let mut idx = 0;

                    (
                        {
                            let storage_idx = indices[idx];
                            idx += 1;
                            (storage[storage_idx]).get_unchecked::<$first>(components_position)
                        },
                        $(
                            {
                                let storage_idx = indices[idx];
                                idx += 1;
                                (storage[storage_idx]).get_unchecked::<$rest>(components_position)
                            }
                        ),+
                    )
                }
            }
        }
    };
}

macro_rules! impl_component_set_all_tuples {
    ($first:ident) => {
        impl_component_set_tuples!($first);
    };
    ($first:ident, $($rest:ident),+) => {
        impl_component_set_tuples!($first, $($rest),+);
        impl_component_set_all_tuples!($($rest),+);
    };
}

impl_component_set_all_tuples!(A, B, C, D, E, F, G, H, I, J);

pub struct ArchetypeQueryView {
    /// Pointer to this specific archetype's component storages vector
    pub storage_ptr: *mut Vec<ComponentStorage>,
    /// Maps how this query's tuple types align with THIS archetype's columns
    pub matching_indices: Vec<usize>,
    /// The row offsets for matching entities inside THIS archetype
    pub entity_positions: Vec<EntityStorageId>,
}

pub struct Query<'a, Q: ComponentSet> {
    /// Views into every archetype that contains our required component subset
    pub archetype_views: Vec<ArchetypeQueryView>,
    pub _marker: PhantomData<&'a Q>,
}

impl<'a, Q: ComponentSet> Query<'a, Q> {
    pub fn new(archetype_views: Vec<ArchetypeQueryView>) -> Self {
        Self {
            archetype_views,
            _marker: PhantomData,
        }
    }

    pub fn iter(&mut self) -> QueryIter<'_, Q> {
        QueryIter {
            views: &self.archetype_views,
            archetype_index: 0,
            entity_index: 0,
            _marker: PhantomData,
        }
    }
}

pub struct QueryIter<'a, Q: ComponentSet> {
    views: &'a [ArchetypeQueryView],
    archetype_index: usize,
    entity_index: usize,
    _marker: PhantomData<Q>,
}

impl<'a, Q: ComponentSet> Iterator for QueryIter<'a, Q> {
    type Item = Q::Fetch<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        while self.archetype_index < self.views.len() {
            let current_view = &self.views[self.archetype_index];

            if self.entity_index < current_view.entity_positions.len() {
                let row_idx = current_view.entity_positions[self.entity_index];
                self.entity_index += 1;

                unsafe {
                    return Some(Q::fetch_from_pointers(
                        current_view.storage_ptr,
                        &current_view.matching_indices,
                        row_idx,
                    ));
                }
            }

            self.archetype_index += 1;
            self.entity_index = 0;
        }

        None
    }
}
