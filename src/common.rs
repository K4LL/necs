use slotmap::{new_key_type};
use std::any::TypeId;

pub(crate) type ComponentSignature = u32;

pub(crate) type EntityStorageId = u32;

new_key_type! { pub struct Entity; }

#[derive(Clone, Copy)]
pub(crate) struct EntityPointer {
    pub archetype: u32,
    pub index: u32,
}

#[derive(Clone, Copy)]
pub(crate) struct TypeMetadata {
    pub size: usize,
    pub alignment: usize,
    pub type_id: TypeId,
}

pub(crate) struct __Internals {}
impl __Internals {
    pub const INITIAL_COMPONENT_STORAGE_CAPACITY: u32 = 64;
}