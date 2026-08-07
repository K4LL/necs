use std::ptr::NonNull;

use crate::common::{ ComponentSignature, __Internals };

pub(crate) struct ComponentStorage {
    storage: *mut u8,
    capacity: u32,
    size: u32,
    item_size: u32,
    signature: ComponentSignature,
}
impl ComponentStorage {
    pub fn new(item_size: u32, item_alignemnt: u32, signature: ComponentSignature) -> Self {
        let initial_capacity = item_size * __Internals::INITIAL_COMPONENT_STORAGE_CAPACITY;

        let layout_result =
            std::alloc::Layout::from_size_align(initial_capacity as usize, item_alignemnt as usize);
        let layout = layout_result.expect("Failed to create layout for ComponentStorage");

        let alloc_result = unsafe { std::alloc::alloc(layout) };
        if alloc_result.is_null() {
            panic!("Failed to allocate for ComponentStorage")
        }

        ComponentStorage {
            storage: alloc_result,
            capacity: initial_capacity,
            size: 0,
            item_size,
            signature,
        }
    }

    pub fn push(&mut self, begin: NonNull<u8>) {
        let end = unsafe { begin.add(self.item_size as usize) };

        let count = unsafe { end.as_ptr().sub(begin.as_ptr() as usize) as usize };

        if self.size + count as u32 >= self.capacity {
            panic!("ComponentStorage is full");
        }

        unsafe {
            self.storage
                .add(self.size as usize)
                .copy_from_nonoverlapping(begin.as_ptr(), count);

            println!(
                "Raw value: {}",
                self.storage.add(self.size as usize).read() as i32
            );
        }

        self.size += count as u32;
    }

    pub fn remove(&mut self, index: u32) {
        let item_index = index * self.item_size;
        if item_index >= self.size {
            panic!("Access out of bounds");
        }
        unsafe {
            let last_ptr =
                NonNull::new(self.storage.add(self.item_size as usize)).unwrap_unchecked();
            let current_ptr =
                NonNull::new(self.storage.add(item_index as usize)).unwrap_unchecked();

            if current_ptr != last_ptr {
                current_ptr.copy_from_nonoverlapping(last_ptr, self.item_size as usize);
            }
        }
    }

    pub fn get<T: 'static>(&self, index: u32) -> Option<&T> {
        let item_index = index * self.item_size;
        if item_index >= self.size {
            return None;
        }
        unsafe { Some((self.storage.add(item_index as usize) as *const T).as_ref_unchecked()) }
    }
    pub fn get_mut<T: 'static>(&mut self, index: u32) -> Option<&mut T> {
        let item_index = index * self.item_size;
        if item_index >= self.size {
            return None;
        }
        unsafe { Some((self.storage.add(item_index as usize) as *mut T).as_mut_unchecked()) }
    }

    pub fn get_unchecked<T: 'static>(&self, index: u32) -> &T {
        let item_index = index * self.item_size;
        unsafe { (self.storage.add(item_index as usize) as *const T).as_ref_unchecked() }
    }
    pub unsafe fn get_mut_unchecked<T: 'static>(&mut self, index: u32) -> &mut T {
        let item_index = index * self.item_size;
        unsafe { (self.storage.add(item_index as usize) as *mut T).as_mut_unchecked() }
    }

    pub fn capacity(&self) -> u32 {
        self.capacity
    }
    pub fn size(&self) -> u32 {
        self.size
    }
    pub fn signature(&self) -> ComponentSignature {
        self.signature
    }
}