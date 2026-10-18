use std::marker::PhantomData;
use std::ptr::NonNull;

use slotmap::{DenseSlotMap, Key};

use crate::ecs::archetype::{Archetype, ArchetypeId};
use crate::ecs::common::{
    Component, ComponentMetadata, Entity, EntityPointer, EntityStorageId
};
use crate::ecs::query::{ ComponentSet, Query, QueryArchetypeView, Read, Write};
use crate::ecs::system::{ Flatten, SystemFunction, SystemRunner, SystemTuple, SystemVisitor,
    SystemWrapper,
};

// World builder that allows chaining
// Currently just support system adding, but I also plan to let it register components and some configurations
pub struct ChainedWorldBuilder<T> {
    systems: T,
}
impl<T: Flatten> ChainedWorldBuilder<T>
where
    T::Output: SystemTuple,
{
    pub fn add_system<F, R, W>(self, system: F) -> ChainedWorldBuilder<(T, SystemWrapper<F, R, W>)>
    where
        F: SystemFunction<F, R, W> + Fn(Query<R, Read>, Query<W, Write>),
        R: ComponentSet,
        W: ComponentSet,
    {
        let wrapped = SystemWrapper {
            system,
            _read_marker: PhantomData,
            _write_marker: PhantomData,
        };

        ChainedWorldBuilder {
            systems: (self.systems, wrapped),
        }
    }

    pub fn init(self) -> World<T::Output> {
        World::init(self.systems.flatten())
    }
}

pub struct World<S> {
    // 'entities' point to the real entities inside an archetype
    // (stored_key, index_to_entity_inside_archetype)
    entities: DenseSlotMap<Entity, EntityPointer>,

    archetypes: Vec<Archetype>,

    empty_archetype_id: ArchetypeId,

    systems: Option<S>,
}
impl World<()> {
    pub const fn new() -> ChainedWorldBuilder<()> {
        ChainedWorldBuilder { systems: () }
    }
}
impl<S: SystemTuple> World<S> {
    fn create_archetype(&mut self, layout: Vec<(ComponentMetadata, u32)>) -> ArchetypeId {
        // This function expects a sorted layout
        assert!(
            layout.is_sorted(),
            "Layout is not sorted. Use create_archetype_and_sort instead"
        );

        // Create a new archetype and push it to the archetype array
        let new_archetype = Archetype::new(layout);
        self.archetypes.push(new_archetype);

        // Return its index
        self.archetypes.len() as ArchetypeId - 1
    }
    #[allow(unused)]
    fn create_archetype_and_sort(
        &mut self,
        mut layout: Vec<(ComponentMetadata, u32)>,
    ) -> ArchetypeId {
        // Sort the layout
        // Here it is safe to use unstable because a layout shouldn't have the same component twice
        layout.sort_unstable();

        // Create a new archetype and push it to the archetype array
        let new_archetype = Archetype::new(layout);
        self.archetypes.push(new_archetype);

        // Return its index
        self.archetypes.len() as ArchetypeId - 1
    }

    fn find_or_create_archetype_with_component<T>(
        &mut self,
        current_archetype_id: u32,
    ) -> ArchetypeId
    where
        T: 'static + Component,
    {
        // Get a reference to the archetype the entity is currently in
        let current_archetype = self
            .archetypes
            .get(current_archetype_id as usize)
            .expect("Failed to get archetype from current id");

        // Create a new component layout
        // It will be the same as the old but with one more component, so clone is used
        let mut new_component_layout = current_archetype.component_layout.clone();

        // Push the new component layout alongside its signature and index
        new_component_layout.push((
            T::METADATA,
            current_archetype.component_storages.len() as u32,
        ));
        // Sort the layout
        // Here it is safe to use unstable because a layout shouldn't have the same component twice
        new_component_layout.sort_unstable();

        // Try to find an archetype with the new layout
        // If it does not exist, create a new one
        // Return the Id of the found/new archetype
        self.archetypes
            .iter()
            .position(|archetype| archetype.component_layout == new_component_layout)
            .unwrap_or_else(|| self.create_archetype(new_component_layout) as usize)
            as ArchetypeId
    }

    fn find_or_create_archetype_without_component<T: Component>(
        &mut self,
        current_archetype_id: u32,
    ) -> ArchetypeId {
        // Get required metadatada from current archetype
        let (new_archetype_mask, mut layout_template, _) = {
            let current_archetype = self
                .archetypes
                .get(current_archetype_id as usize)
                .expect("Failed to get archetype from current id");

            (
                current_archetype.archetype_mask & !T::METADATA.mask,
                current_archetype.component_layout.clone(),
                current_archetype.component_storages.len() as u32,
            )
        };

        // Remove the requested component
        unsafe {
            layout_template.remove(
                layout_template
                    .iter()
                    .position(|c| c.0.mask == T::METADATA.mask)
                    .unwrap_unchecked(),
            )
        };

        // Try to find a pre existing archetype with the new mask
        if let Some(index) = self
            .archetypes
            .iter()
            .position(|archetype| archetype.archetype_mask == new_archetype_mask)
        {
            return index as ArchetypeId;
        }

        // Fallback
        // Create an archetype with the new layout
        self.create_archetype(layout_template) as ArchetypeId
    }

    fn delete_entity_from_archetype_using_pointer(&mut self, entity_ptr: EntityPointer) {
        let archetype = unsafe {
            self.archetypes
                .get_unchecked_mut(entity_ptr.archetype as usize)
        };
        // Remove the entity from the archetype
        archetype.remove_entity_swap(entity_ptr.index);

        // After the swap-remove, whatever entity used to be last now sits at entity_ptr.index
        // (unless we removed what was already the last slot, in which case nothing moved).
        let moved_archetype_entity_ptr = archetype.entity_storages.get(entity_ptr.index as usize);
        if moved_archetype_entity_ptr.is_none() {
            return;
        }

        unsafe {
            let moved_archetype_entity_slot = {
                self.entities
                    .get_mut(moved_archetype_entity_ptr.unwrap_unchecked().0)
                    .expect("Moved archetype entity not found on world storage")
            };
            moved_archetype_entity_slot.index = entity_ptr.index;
        }
    }

    fn init(systems: S) -> Self {
        let mut entities = DenseSlotMap::with_key();

        entities.reserve(1024);

        let mut archetypes = Vec::new();
        archetypes.reserve(1024);

        let mut world = Self {
            entities,
            archetypes,
            empty_archetype_id: 0,
            systems: Some(systems),
        };

        // Create an empty archetype to store entities without components
        world.archetypes.push(Archetype::new(Vec::new()));

        world
    }

    pub fn spawn(&mut self) -> Entity {
        // Get empty archetype reference
        let empty_archetype = self
            .archetypes
            .get_mut(self.empty_archetype_id as usize)
            .expect("Could not find empty archetype");
        // Get entity storages reference
        let entity_storages = &mut empty_archetype.entity_storages;

        // Push a new entity storage with a null entity and index 0, which will be updated later
        entity_storages.push((Entity::null(), 0));

        // Insert the new entity into the entities slotmap, pointing to the empty archetype and the last index of the entity storages
        let public_entity = self.entities.insert(EntityPointer {
            archetype: self.empty_archetype_id as u32,
            index: entity_storages.len() as u32 - 1,
        });

        // Update the last entity storage with the actual public entity
        // Here it is safe to use unchecked because we just pushed a new element to the vector, so the item is valid
        let entity_storage_last = unsafe { entity_storages.last_mut().unwrap_unchecked() };
        entity_storage_last.0 = public_entity;

        public_entity
    }

    pub fn despawn(&mut self, entity: Entity) {
        // Delete from archetype
        self.delete_entity_from_archetype_using_pointer(
            *self.entities.get(entity).expect("Failed to get entity"),
        );

        // Remove the entity from the entities slotmap
        self.entities.remove(entity);
    }

    pub fn add_component<T: Component>(&mut self, entity: Entity, component: T) {
        let entity_pointer = *self
            .entities
            .get(entity)
            .expect("Could not find entity pointer for the given entity");

        // Get or create an archetype that fits the entity
        let new_archetype_id =
            self.find_or_create_archetype_with_component::<T>(entity_pointer.archetype);

        // Get the position of the entity components within the now old archetype
        let old_components_position = self
            .archetypes
            .get(entity_pointer.archetype as usize)
            .unwrap()
            .entity_storages
            .get(entity_pointer.index as usize)
            .unwrap()
            .1;

        // Create an array with the begin iterator and signature of each component
        // These will be later copied into the new archetype
        let mut copy_list = Vec::new();
        // Iterate over the component storages
        for storage in self
            .archetypes
            .get(entity_pointer.archetype as usize)
            .unwrap()
            .component_storages
            .iter()
        {
            // Get the begin iterator within the now old storage
            let old_component_begin = NonNull::from_ref(
                storage
                    .get(old_components_position)
                    .expect("Failed to get old component from archetype"),
            );
            // Push the begin iterator alongside the component signature to the copy list
            copy_list.push((old_component_begin, *storage.mask()));
        }

        // Get the begin iterator for the new component and push it into the copy list
        let new_component_begin = NonNull::from_ref(&component).cast::<u8>();
        copy_list.push((new_component_begin, T::METADATA.mask));

        // Add the entity to its new archetype, consuming the copy list
        self.archetypes
            .get_mut(new_archetype_id as usize)
            .unwrap()
            .add_entity(entity, copy_list);

        // Update the entity handles
        unsafe {
            // Point the Entity to the new archetype
            self.entities.get_unchecked_mut(entity).archetype = new_archetype_id;
            // Point the Entity to its updated index
            self.entities.get_unchecked_mut(entity).index = self
                .archetypes
                .get_unchecked_mut(new_archetype_id as usize)
                .entity_storages
                .len() as u32
                - 1;
        }

        // Delete from old archetype (copied pointer needed)
        self.delete_entity_from_archetype_using_pointer(entity_pointer);
    }

    pub fn remove_component<T: Component>(&mut self, entity: Entity) -> () {
        let entity_pointer = *self
            .entities
            .get(entity)
            .expect("Failed to get entity for given id");

        let new_archetype_id =
            self.find_or_create_archetype_without_component::<T>(entity_pointer.archetype);

        // Get the position of the entity components within the now old archetype
        let old_components_position = self
            .archetypes
            .get(entity_pointer.archetype as usize)
            .unwrap()
            .entity_storages
            .get(entity_pointer.index as usize)
            .unwrap()
            .1;

        // Create an array with the begin iterator and signature of each component
        // These will be later copied into the new archetype
        let mut copy_list = Vec::new();
        // Iterate over the component storages
        for storage in self
            .archetypes
            .get(entity_pointer.archetype as usize)
            .unwrap()
            .component_storages
            .iter()
        {
            if storage.mask() == &T::METADATA.mask {
                continue;
            }

            // Get the begin iterator within the now old storage
            let old_component_begin = NonNull::from_ref(
                storage
                    .get(old_components_position)
                    .expect("Failed to get old component from archetype"),
            );
            // Push the begin iterator alongside the component signature to the copy list
            copy_list.push((old_component_begin, *storage.mask()));
        }

        // Add the entity to its new archetype, consuming the copy list
        self.archetypes
            .get_mut(new_archetype_id as usize)
            .unwrap()
            .add_entity(entity, copy_list);

        // Update the entity handles
        unsafe {
            // Point the Entity to the new archetype
            self.entities.get_unchecked_mut(entity).archetype = new_archetype_id;
            // Point the Entity to its updated index
            self.entities.get_unchecked_mut(entity).index = self
                .archetypes
                .get_unchecked_mut(new_archetype_id as usize)
                .entity_storages
                .len() as u32
                - 1;
        }

        // Delete from old archetype (copied pointer needed)
        self.delete_entity_from_archetype_using_pointer(entity_pointer);
    }

    pub fn get_component<T: Component>(&self, entity: Entity) -> &T {
        let entity_pointer = self
            .entities
            .get(entity)
            .expect("Failed to get entity for given id");

        let archetype = self
            .archetypes
            .get(entity_pointer.archetype as usize)
            .expect("Failed to get archetype from entity pointer");

        if (archetype.archetype_mask.union(&T::METADATA.mask)) != archetype.archetype_mask {
            panic!("No component with the given type was found")
        }

        let component_position = archetype
            .entity_storages
            .get(entity_pointer.index as usize)
            .expect("Failed to get internal entity from entity pointer");

        for component_storage in archetype.component_storages.iter() {
            // If the found storage isn't the needed storage, continue
            if *component_storage.mask() != T::METADATA.mask {
                continue;
            }

            // If the found storage is the one needed, return a reference to the component inside it
            return component_storage
                .get(component_position.1)
                .expect("Component not found for given index");
        }

        panic!("No component with the given type was found")
    }

    pub fn query<R: ComponentSet>(&self) -> Query<R, Read> {
        // Create archetype view array
        let mut archetype_views = Vec::new();

        // Iterate through each existing archetype
        for archetype in self.archetypes.iter() {
            // Check if the archetype contains the queried components
            if archetype.archetype_mask.contains(&R::MASK) {
                // Get a ref to the component storages from the archetype
                let storages = &archetype.component_storages;
                // Create an array, where each index points to the correct component storage, as it can be queried in any order
                let matching_indices: Vec<usize> = R::COMPONENTS
                    .iter()
                    .map(|meta| {
                        storages
                            .iter()
                            .position(|ptr| *ptr.mask() == meta.mask)
                            .expect("Archetype missing a component required by Query")
                    })
                    .collect();

                // Get the position where the entity components are currently sitting in
                let entity_positions: Vec<EntityStorageId> = archetype
                    .entity_storages
                    .iter()
                    .map(|(_, row_idx)| *row_idx)
                    .collect();

                if entity_positions.is_empty() {
                    continue;
                }

                let storage_ptr = std::ptr::from_ref(&archetype.component_storages);

                // Push to the archetype view array
                archetype_views.push(QueryArchetypeView::<Read> {
                    storage_ptr,
                    matching_indices,
                    entity_positions,
                });
            }
        }

        // Create and return the query
        Query::new(archetype_views)
    }

    pub fn query_mut<W: ComponentSet>(&mut self) -> Query<W, Write> {
        // Create archetype view array
        let mut archetype_views = Vec::new();

        // Iterate through each existing archetype
        for archetype in self.archetypes.iter_mut() {
            // Check if the archetype contains the queried components
            if archetype.archetype_mask.contains(&W::MASK) {
                // Get a ref to the component storages from the archetype
                let storages = &archetype.component_storages;
                // Create an array, where each index points to the correct component storage, as it can be queried in any order
                let matching_indices: Vec<usize> = W::COMPONENTS
                    .iter()
                    .map(|meta| {
                        storages
                            .iter()
                            .position(|ptr| *ptr.mask() == meta.mask)
                            .expect("Archetype missing a component required by Query")
                    })
                    .collect();

                // Get the position where the entity components are currently sitting in
                let entity_positions: Vec<EntityStorageId> = archetype
                    .entity_storages
                    .iter()
                    .map(|(_, row_idx)| *row_idx)
                    .collect();

                if entity_positions.is_empty() {
                    continue;
                }

                let storage_ptr = std::ptr::from_mut(&mut archetype.component_storages);

                // Push to the archetype view array
                archetype_views.push(QueryArchetypeView::<Write> {
                    storage_ptr,
                    matching_indices,
                    entity_positions,
                });
            }
        }

        // Create and return the query
        Query::new(archetype_views)
    }

    pub fn run_systems(&mut self) {
        // Take ownership of systems
        if let Some(mut systems) = self.systems.take() {
            let mut runner = SystemRunner;

            // Run systems passing
            systems.accept_visitor(&mut runner, self);

            // Put systems back
            self.systems = Some(systems);
        }
    }

    pub fn run_scheduler<V: SystemVisitor>(&mut self, scheduler: &mut V) {
        if let Some(mut systems) = self.systems.take() {
            systems.accept_visitor(scheduler, self);
            self.systems = Some(systems);
        }
    }
}
