use std::ptr::NonNull;

use crate::common::{ Entity, EntityStorageId, ComponentSignature };
use crate::component_storage::ComponentStorage;
use crate::world::World;

pub(crate) type ArchetypeId = u32;

pub(crate) struct Archetype {
    // (back_reference, row)
    pub entity_storages: Vec<(Entity, EntityStorageId)>,
    pub component_storages: std::vec::Vec<ComponentStorage>,
    // (component_signature, index)
    pub component_layout: std::vec::Vec<(ComponentSignature, u32)>,
}
impl Archetype {
    pub fn new(world: &World, layout: Vec<(ComponentSignature, u32)>) -> Self {
        let mut component_storages = Vec::new();
        for component_layout in layout.iter() {
            let type_metadata = world
                .get_type_metadata_for_signature(component_layout.0)
                .expect("Failed to get metadata for given signature");
            component_storages.push(ComponentStorage::new(
                type_metadata.size as u32,
                type_metadata.alignment as u32,
                component_layout.0,
            ));
        }

        Self {
            entity_storages: Vec::new(),
            component_storages: component_storages,
            component_layout: layout,
        }
    }

    pub fn add_entity(
        &mut self,
        entity: Entity,
        components: Vec<(NonNull<u8>, ComponentSignature)>,
    ) -> EntityStorageId {
        assert!(
            components.len() == self.component_layout.len(),
            "Component count does not match archetype layout"
        );

        for it in components.iter() {
            let (ptr, signature) = it;

            let component_index = self
                .component_layout
                .iter()
                .position(|(sig, _)| sig == signature)
                .expect("Component signature not found in archetype layout");

            let component_storage = &mut self.component_storages[component_index];

            component_storage.push(*ptr);
        }
        self.entity_storages
            .push((entity, self.entity_storages.len() as u32));

        self.entity_storages.len() as EntityStorageId - 1
    }

    pub fn remove_entity_swap(&mut self, entity_storage_id: EntityStorageId) {
        for storage in self.component_storages.iter_mut() {
            storage.remove(entity_storage_id);
        }

        let entity_storages_len = self.entity_storages.len();
        if entity_storages_len > 1 {
            let last_entity = self.entity_storages.last_mut();
            if last_entity.is_some() {
                unsafe { last_entity.unwrap_unchecked().1 = entity_storage_id };
            }
        }
        self.entity_storages.swap_remove(entity_storage_id as usize);
    }
}