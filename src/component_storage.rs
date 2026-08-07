use std::ptr::NonNull;

use crate::common::{__Internals, ComponentSignature};

pub(crate) struct ComponentStorage {
    storage: *mut u8,
    capacity: u32,
    size: u32,
    item_size: u32,
    signature: ComponentSignature,
}
impl ComponentStorage {
    pub fn new(item_size: u32, item_alignemnt: u32, signature: ComponentSignature) -> Self {
        // Calculate initial capacity
        let initial_capacity = item_size * __Internals::INITIAL_COMPONENT_STORAGE_CAPACITY;

        // Create layout based on the initial capacity and item alignemnt
        let layout_result =
            std::alloc::Layout::from_size_align(initial_capacity as usize, item_alignemnt as usize);
        let layout = layout_result.expect("Failed to create layout for ComponentStorage");

        // Allocate memory
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
        // Get the end of the item using the item size
        let end = unsafe { begin.add(self.item_size as usize) };

        // How many bytes will be writen
        let count = unsafe { end.as_ptr().sub(begin.as_ptr() as usize) as usize };

        if self.size + count as u32 >= self.capacity {
            panic!("ComponentStorage is full");
        }

        // Add the current size to storage, which is the current end
        // Then copy using the iterator and the byte count calculated before
        unsafe {
            self.storage
                .add(self.size as usize)
                .copy_from_nonoverlapping(begin.as_ptr(), count);
        }

        // Update size
        self.size += count as u32;
    }

    /// Removes the element at the given index by swapping it with the last element.
    pub fn remove(&mut self, index: u32) {
        if self.size == 0 {
            panic!("Trying to remove an item from an empty storage")
        }

        // Calculate item index
        let item_index = index * self.item_size;
        if item_index >= self.size {
            panic!("Access out of bounds");
        }
        unsafe {
            // Get the last storage item
            let last_ptr =
                NonNull::new(self.storage.add(self.item_size as usize)).unwrap_unchecked();
            // Get the target item
            let current_ptr =
                NonNull::new(self.storage.add(item_index as usize)).unwrap_unchecked();

            // If the target is not the last item, copy the last item onto the target item
            if current_ptr != last_ptr {
                current_ptr.copy_from_nonoverlapping(last_ptr, self.item_size as usize);
            }
        }

        // Update size
        self.size -= self.item_size;
    }

    pub fn get<T: 'static>(&self, index: u32) -> Option<&T> {
        // Calculate the real index
        let item_index = index * self.item_size;
        // Return None if out of bounds
        if item_index >= self.size {
            return None;
        }
        // Get the item position by adding the index to the start, then convert it to a ref.
        // Here it is safe to use as_ref_unchecked because the storage is guaranteed to be initialized.
        unsafe { Some((self.storage.add(item_index as usize) as *const T).as_ref_unchecked()) }
    }

    pub fn get_mut<T: 'static>(&mut self, index: u32) -> Option<&mut T> {
        // Calculate the real index
        let item_index = index * self.item_size;
        // Return None if out of bounds
        if item_index >= self.size {
            return None;
        }
        // Get the item position by adding the index to the start, then convert it to a ref.
        // Here it is safe to use as_ref_unchecked because the storage is guaranteed to be initialized.
        unsafe { Some((self.storage.add(item_index as usize) as *mut T).as_mut_unchecked()) }
    }

    pub fn get_unchecked<T: 'static>(&self, index: u32) -> &T {
        // Calculate the real index
        let item_index = index * self.item_size;
        // Get the item position by adding the index to the start, then convert it to a ref.
        // Here it is safe to use as_ref_unchecked because the storage is guaranteed to be initialized.
        unsafe { (self.storage.add(item_index as usize) as *const T).as_ref_unchecked() }
    }
    pub unsafe fn get_mut_unchecked<T: 'static>(&mut self, index: u32) -> &mut T {
        // Calculate the real index
        let item_index = index * self.item_size;
        // Get the item position by adding the index to the start, then convert it to a ref.
        // Here it is safe to use as_ref_unchecked because the storage is guaranteed to be initialized.
        unsafe { (self.storage.add(item_index as usize) as *mut T).as_mut_unchecked() }
    }

    pub fn capacity(&self) -> u32 {
        // Returns how many items fits in
        self.capacity / self.item_size
    }
    pub fn size(&self) -> u32 {
        // Returns how many items there currently are
        self.size / self.item_size
    }
    pub fn signature(&self) -> ComponentSignature {
        self.signature
    }
}
