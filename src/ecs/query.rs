use std::marker::PhantomData;

use crate::ecs::{
    common::{Component, ComponentMask, ComponentMetadata, EntityStorageId}, component_storage::ComponentStorage,
};

pub trait ComponentSet {
    const LEN: usize;
    const MASK: ComponentMask;

    const COMPONENTS: &'static [&'static ComponentMetadata];

    type Fetch<'a>;

     unsafe fn fetch_from_pointers<'a>(
        ptrs_storage: *const Vec<ComponentStorage>,
        indices: &[usize],
        components_position: EntityStorageId,
    ) -> Self::Fetch<'a>;
    unsafe fn fetch_from_pointers_mut<'a>(
        ptrs_storage: *mut Vec<ComponentStorage>,
        indices: &[usize],
        components_position: EntityStorageId,
    ) -> Self::Fetch<'a>;
}
impl ComponentSet for () {
    const LEN: usize = 0;
    const MASK: ComponentMask = ComponentMask { words: [0; 4] };
    const COMPONENTS: &'static [&'static ComponentMetadata] = &[];

    type Fetch<'a> = ();

    unsafe fn fetch_from_pointers<'a>(
        _ptrs_storage: *const Vec<ComponentStorage>,
        _indices: &[usize],
        _components_position: EntityStorageId,
    ) -> Self::Fetch<'a> {
        ()
    }
    unsafe fn fetch_from_pointers_mut<'a>(
        _ptrs_storage: *mut Vec<ComponentStorage>,
        _indices: &[usize],
        _components_position: EntityStorageId,
    ) -> Self::Fetch<'a> {
        ()
    }
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

            #[allow(unused_variables)]
            unsafe fn fetch_from_pointers<'a>(
                ptrs_storage: *const Vec<ComponentStorage>,
                indices: &[usize],
                components_position: EntityStorageId
            ) -> Self::Fetch<'a> {
                unsafe {
                    let storage = &*ptrs_storage;
                    (storage[0]).get_unchecked::<$first>(components_position)
                }
            }
            #[allow(unused_variables)]
            unsafe fn fetch_from_pointers_mut<'a>(
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
                ptrs_storage: *const Vec<ComponentStorage>,
                indices: &[usize],
                components_position: EntityStorageId,
            ) -> Self::Fetch<'a> {
                unsafe {
                    let storage = &*ptrs_storage;
                    let mut iter = indices.iter();

                    (
                        {
                            let storage_idx = *iter.next().unwrap();
                            (storage[storage_idx]).get_unchecked::<$first>(components_position)
                        }, // <--- Comma separating the first element from the rest
                        $(
                            {
                                let storage_idx = *iter.next().unwrap();
                                (storage[storage_idx]).get_unchecked::<$rest>(components_position)
                            }
                        ),+
                    )
                }
            }

            unsafe fn fetch_from_pointers_mut<'a>(
                ptrs_storage: *mut Vec<ComponentStorage>,
                indices: &[usize],
                components_position: EntityStorageId,
            ) -> Self::Fetch<'a> {
                unsafe {
                    let storage = &*ptrs_storage;
                    let mut iter = indices.iter();

                    (
                        {
                            let storage_idx = *iter.next().unwrap();
                            (storage[storage_idx]).get_unchecked::<$first>(components_position)
                        },
                        $(
                            {
                                let storage_idx = *iter.next().unwrap();
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

pub struct Read {}
pub struct Write {}

pub trait AccessMode {}
pub trait WriteAccessMode {}
pub trait ReadAccessMode {}

impl AccessMode for Read {}
impl AccessMode for Write {}

impl WriteAccessMode for Write {}
impl ReadAccessMode for Read {}

pub trait QueryStorageState {
    type Storage: Copy;

    unsafe fn fetch<'a, Q: ComponentSet>(
        storage_ptr: Self::Storage,
        indices: &[usize],
        components_position: EntityStorageId,
    ) -> Q::Fetch<'a>;
}

impl QueryStorageState for Read {
    type Storage = *const Vec<ComponentStorage>;

    unsafe fn fetch<'a, Q: ComponentSet>(
        storage_ptr: Self::Storage,
        indices: &[usize],
        components_position: EntityStorageId,
    ) -> Q::Fetch<'a> {
        Q::fetch_from_pointers(storage_ptr, indices, components_position)
    }
}

impl QueryStorageState for Write {
    type Storage = *mut Vec<ComponentStorage>;

    unsafe fn fetch<'a, Q: ComponentSet>(
        storage_ptr: Self::Storage,
        indices: &[usize],
        components_position: EntityStorageId,
    ) -> Q::Fetch<'a> {
        Q::fetch_from_pointers_mut(storage_ptr, indices, components_position)
    }
}

pub(crate) struct QueryArchetypeView<S: QueryStorageState> {
    pub storage_ptr: S::Storage,
    pub matching_indices: Vec<usize>,
    pub entity_positions: Vec<EntityStorageId>,
}

pub struct Query<Q: ComponentSet, S: QueryStorageState> {
    /// Views into every archetype that contains our required component subset
    pub(crate) archetype_views: Vec<QueryArchetypeView<S>>,
    pub(crate) _marker: PhantomData<Q>,
}

impl<Q: ComponentSet, S: QueryStorageState> Query<Q, S> {
    pub(crate) fn new(archetype_views: Vec<QueryArchetypeView<S>>) -> Self {
        Self {
            archetype_views,
            _marker: PhantomData,
        }
    }

    pub fn iter(&mut self) -> QueryIter<'_, Q, S> {
        QueryIter {
            views: &self.archetype_views,
            archetype_index: 0,
            entity_index: 0,
            _marker: PhantomData,
        }
    }
}

pub struct QueryIter<'a, Q: ComponentSet, S: QueryStorageState> {
    views: &'a [QueryArchetypeView<S>],
    archetype_index: usize,
    entity_index: usize,
    _marker: PhantomData<Q>,
}

impl<'a, Q, S> Iterator for QueryIter<'a, Q, S> 
where 
    Q: ComponentSet, 
    S: QueryStorageState,
{
    type Item = Q::Fetch<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        while self.archetype_index < self.views.len() {
            let current_view = &self.views[self.archetype_index];

            if self.entity_index < current_view.entity_positions.len() {
                let row_idx = current_view.entity_positions[self.entity_index];
                self.entity_index += 1;

                unsafe {
                    return Some(S::fetch::<Q>(
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
