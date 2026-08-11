use std::ptr::NonNull;

use crate::ecs::common::__Internals;
use crate::ecs::common::{ComponentMask, ComponentMetadata, Entity, EntityStorageId};
use crate::ecs::component_storage::ComponentStorage;
use crate::ecs::world::World;

pub(crate) type ArchetypeId = u32;

pub(crate) struct Archetype {
    // (back_reference, row)
    pub entity_storages: Vec<(Entity, EntityStorageId)>,
    pub component_storages: std::vec::Vec<ComponentStorage>,
    // (component_signature, index)
    pub component_layout: std::vec::Vec<(ComponentMetadata, u32)>,
    pub archetype_mask: ComponentMask,
}
impl Archetype {
    pub fn new(world: &World, layout: Vec<(ComponentMetadata, u32)>) -> Self {
        // Create a component storage array
        let mut component_storages = Vec::new();
        let mut mask = ComponentMask {
            words: [0; __Internals::COMPONENT_BITSET_WORDS as usize],
        };
        for component_layout in layout.iter() {
            // Get type metadata for the given signature
            // This is very important, as this metadata is needed to create the storage
            // Create a new component storage, then push it to the storage array
            let new_component_storage = ComponentStorage::new(
                component_layout.0.size as u32,
                component_layout.0.alignment as u32,
                component_layout.0.mask.clone(),
            );
            component_storages.push(new_component_storage);

            mask = mask.union(&component_layout.0.mask);
        }

        Self {
            entity_storages: Vec::new(),
            component_storages: component_storages,
            component_layout: layout,
            archetype_mask: mask,
        }
    }

    pub fn add_entity(
        &mut self,
        entity: Entity,
        components: Vec<(NonNull<u8>, ComponentMask)>,
    ) -> EntityStorageId {
        assert!(
            components.len() == self.component_layout.len(),
            "Component count does not match archetype layout"
        );

        // Copy each index and begin iterator from the old components to the new storage
        for it in components.iter() {
            // Get begin iterator and signature
            let (ptr, mask) = it;

            // Get the component storage index
            let component_index = self
                .component_layout
                .iter()
                .position(|(sig, _)| sig.mask == *mask)
                .expect("Component signature not found in archetype layout");

            // Get the component storage based on the index
            let component_storage = &mut self.component_storages[component_index];
            // Push the component to the storage
            component_storage.push(*ptr);
        }
        // Push a copy of the Entity and the current component position to the entity storage
        // Not subtracting one from len because the entity is being pushed, so -1 would be the last entity
        self.entity_storages
            .push((entity, self.entity_storages.len() as u32));

        // Now subtract one because the entity was already pushed
        self.entity_storages.len() as EntityStorageId - 1
    }

    pub fn remove_entity_swap(&mut self, entity_storage_id: EntityStorageId) {
        // Remove all entity's components from their storages
        for storage in self.component_storages.iter_mut() {
            storage.remove(entity_storage_id);
        }

        let entity_storages_len = self.entity_storages.len();
        // If there is more than one entity, swap the last entity component index with the removed index, as it was a swap remove
        if entity_storages_len > 1 {
            let last_entity = self.entity_storages.last_mut();
            if last_entity.is_some() {
                unsafe { last_entity.unwrap_unchecked().1 = entity_storage_id };
            }
        }
        // Then remove the entity from the registry using its own id
        // For me this function is very messy so I will write it
        // ITS ID DIDN'T CHANGE, JUST THE LAST ELEMENT STORAGE ID AND ITS POSITION
        self.entity_storages.swap_remove(entity_storage_id as usize);
    }
}
