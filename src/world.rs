use std::any::TypeId;
use std::ptr::NonNull;

use slotmap::{ DenseSlotMap, Key };

use crate::common::{ ComponentSignature, TypeMetadata, Entity, EntityPointer };
use crate::archetype::{ Archetype, ArchetypeId };

pub struct World {
    component_signatures: Vec<(ComponentSignature, TypeMetadata)>,

    // 'entities' point to the real entities inside an archetype
    // (stored_key, index_to_entity_inside_archetype)
    entities: DenseSlotMap<Entity, EntityPointer>,

    archetypes: Vec<Archetype>,

    empty_archetype_id: ArchetypeId,
}
impl World {
    pub(crate) fn get_signature_for_component<T: 'static>(&self) -> Option<ComponentSignature> {
        let type_id = TypeId::of::<T>();

        self.component_signatures
            .iter()
            .find(|(_, id)| id.type_id == type_id)
            .map(|(sig, _)| *sig)
    }

    pub(crate) fn get_or_create_signature_for_component<T: 'static>(&mut self) -> ComponentSignature {
        let type_id = TypeId::of::<T>();

        self.component_signatures
            .iter()
            .find(|(_, id)| id.type_id == type_id)
            .map(|(sig, _)| *sig)
            .unwrap_or_else(|| self.register_component::<T>())
    }

    pub(crate) fn get_type_metadata_for_signature(
        &self,
        signature: ComponentSignature,
    ) -> Option<TypeMetadata> {
        self.component_signatures
            .iter()
            .find(|&t| t.0 == signature)
            .map(|t| t.1)
    }

    fn create_archetype(&mut self, layout: Vec<(ComponentSignature, u32)>) -> ArchetypeId {
        assert!(
            layout.is_sorted(),
            "Layout is not sorted. Use create_archetype_and_sort instead"
        );

        let new_archetype = Archetype::new(self, layout);
        self.archetypes.push(new_archetype);

        self.archetypes.len() as ArchetypeId - 1
    }
    fn create_archetype_and_sort(
        &mut self,
        mut layout: Vec<(ComponentSignature, u32)>,
    ) -> ArchetypeId {
        layout.sort_unstable();
        self.archetypes.push(Archetype::new(self, layout));

        self.archetypes.len() as ArchetypeId - 1
    }

    fn find_or_create_archetype_with_component<T: 'static>(
        &mut self,
        current_archetype_id: usize,
    ) -> ArchetypeId {
        let new_component_signature = self.get_or_create_signature_for_component::<T>();

        let current_archetype = self
            .archetypes
            .get(current_archetype_id)
            .expect("Failed to get archetype from current id");

        let mut new_component_layout = current_archetype.component_layout.clone();

        new_component_layout.push((
            new_component_signature,
            current_archetype.component_storages.len() as u32,
        ));
        new_component_layout.sort_unstable();

        self.archetypes
            .iter()
            .position(|archetype| archetype.component_layout == new_component_layout)
            .unwrap_or_else(|| self.create_archetype(new_component_layout) as usize)
            as ArchetypeId
    }

    fn delete_entity_from_archetype_using_pointer(&mut self, entity_ptr: EntityPointer) {
        let archetype = unsafe { self.archetypes.get_unchecked_mut(entity_ptr.archetype as usize) };
        archetype.remove_entity_swap(entity_ptr.index);

        let last_archetype_entity_ptr = archetype.entity_storages.last();
        if last_archetype_entity_ptr.is_none() {
            return;
        }

        unsafe {
            let last_archetype_entity_slot = {
                self.entities
                    .get_mut(last_archetype_entity_ptr.unwrap_unchecked().0)
                    .expect("Last archetype entity not found on world storage")
            };
            last_archetype_entity_slot.index = last_archetype_entity_ptr.unwrap_unchecked().1;
        }
    }

    pub fn new() -> Self {
        let mut entities = DenseSlotMap::with_key();

        entities.reserve(1024);

        let mut archetypes = Vec::new();
        archetypes.reserve(1024);

        let mut world = Self {
            component_signatures: Vec::new(),
            entities,
            archetypes,
            empty_archetype_id: 0,
        };

        // Create an empty archetype to store entities without components
        world.archetypes.push(Archetype::new(&world, Vec::new()));

        world
    }

    pub fn register_component<T: 'static>(&mut self) -> ComponentSignature {
        let type_metadata = TypeMetadata {
            size: size_of::<T>(),
            alignment: align_of::<T>(),
            type_id: TypeId::of::<T>(),
        };

        if let Some((signature, _)) = self
            .component_signatures
            .iter()
            .find(|(_, id)| id.type_id == type_metadata.type_id)
        {
            return *signature;
        }

        let new_signature = self.component_signatures.len() as ComponentSignature;
        self.component_signatures
            .push((new_signature, type_metadata));

        new_signature
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

    pub fn add_component<T: 'static>(&mut self, entity: Entity, component: T) {
        let entity_pointer = *self
            .entities
            .get(entity)
            .expect("Could not find entity pointer for the given entity");

        let new_archetype_id =
            self.find_or_create_archetype_with_component::<T>(entity_pointer.archetype as usize);

        let old_components_position = self
            .archetypes
            .get(entity_pointer.archetype as usize)
            .unwrap()
            .entity_storages
            .get(entity_pointer.index as usize)
            .unwrap()
            .1;

        let mut new_components = Vec::new();
        {
            for storage in self
                .archetypes
                .get(entity_pointer.archetype as usize)
                .unwrap()
                .component_storages
                .iter()
            {
                let old_component_begin = NonNull::from_ref(
                    storage
                        .get(old_components_position)
                        .expect("Failed to get old component from archetype"),
                );
                new_components.push((old_component_begin, storage.signature()));
            }
        }

        let new_component_begin = NonNull::from_ref(&component).cast::<u8>();
        new_components.push((
            new_component_begin,
            self.get_or_create_signature_for_component::<T>(),
        ));

        self.archetypes
            .get_mut(new_archetype_id as usize)
            .unwrap()
            .add_entity(entity, new_components);

        unsafe {
            self.entities.get_unchecked_mut(entity).archetype = new_archetype_id;
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

    pub fn get_component<T: 'static>(&self, entity: Entity) -> &T {
        let entity_pointer = self
            .entities
            .get(entity)
            .expect("Failed to get entity for given id");

        let archetype = self
            .archetypes
            .get(entity_pointer.archetype as usize)
            .expect("Failed to get archetype from entity pointer");

        let component_position = archetype
            .entity_storages
            .get(entity_pointer.index as usize)
            .expect("Failed to get internal entity from entity pointer");
        let component_signature = self
            .get_signature_for_component::<T>()
            .expect("Component is not registered");

        for component_storage in archetype.component_storages.iter() {
            if component_storage.signature() != component_signature {
                continue;
            }

            return component_storage
                .get(component_position.1)
                .expect("Component not found for given index");
        }

        panic!("No component with the given type was found")
    }
}