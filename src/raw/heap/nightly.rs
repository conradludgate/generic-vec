use crate::raw::{AllocError, AllocResult, Storage, StorageWithCapacity};

use core::{alloc::Layout, ptr::NonNull};
use std::{alloc::handle_alloc_error, mem::MaybeUninit};

use std::alloc::Allocator;

enum OnFailure {
    Abort,
    Error,
}

type Heap<T, A> = Box<[MaybeUninit<T>], A>;

/// Create a new `Heap<T>`storage from the given pointer and capacity
///
/// # Safety
///
/// If the capacity is non-zero
/// * You must have allocated the pointer from the given allocator
/// * The pointer must be valid to read-write for the range `ptr..ptr.add(capacity)`
pub(crate) unsafe fn box_from_raw_parts_in<T, A: Allocator>(
    ptr: NonNull<T>,
    capacity: usize,
    allocator: A,
) -> Heap<T, A> {
    unsafe {
        let ptr = std::ptr::slice_from_raw_parts_mut(ptr.as_ptr().cast(), capacity);
        Box::from_raw_in(ptr, allocator)
    }
}

/// Convert a `Heap` storage into a pointer and capacity, without
/// deallocating the storage
pub(crate) fn box_into_raw_parts_with_alloc<T, A: Allocator>(b: Heap<T, A>) -> (NonNull<T>, usize, A) {
    let (ptr, alloc) = Box::into_raw_with_allocator(b);
    unsafe {
        let (ptr, capacity) = ptr.to_raw_parts();
        (NonNull::new_unchecked(ptr.cast()), capacity, alloc)
    }
}

unsafe impl<T, A: Allocator> Storage for Heap<T, A> {
    type Item = T;

    fn reserve(&mut self, new_capacity: usize) {
        if self.len() < new_capacity {
            let _ = reserve_slow(self, new_capacity, OnFailure::Abort);
        }
    }

    fn try_reserve(&mut self, new_capacity: usize) -> AllocResult {
        if self.len() < new_capacity {
            reserve_slow(self, new_capacity, OnFailure::Error)
        } else {
            Ok(())
        }
    }
}

unsafe impl<T, A: Default + Allocator> StorageWithCapacity for Heap<T, A> {
    fn with_capacity(cap: usize) -> Self { Box::new_uninit_slice_in(cap, A::default()) }
}

#[cold]
#[inline(never)]
fn reserve_slow<T, A: Allocator>(b: &mut Heap<T, A>, new_capacity: usize, on_failure: OnFailure) -> AllocResult {
    assert!(new_capacity > b.len());

    // taking a copy of the box so we can get it's contents and then update it later
    // Safety:
    // we forget the box just as soon we we copy it, so we have no risk of double-free
    let (ptr, cap, alloc) = unsafe { box_into_raw_parts_with_alloc(std::ptr::read(b)) };

    // grow by at least doubling
    let new_capacity = new_capacity
        .max(cap.checked_mul(2).expect("Could not grow further"))
        .max(super::INIT_ALLOC_CAPACITY);
    let layout = Layout::new::<T>().repeat(new_capacity).expect("Invalid layout").0;

    let ptr = if cap == 0 {
        unsafe { alloc.allocate(layout) }
    } else {
        let new_layout = layout;
        let old_layout = Layout::new::<T>().repeat(cap).expect("Invalid layout").0;

        unsafe { alloc.grow(ptr.cast(), old_layout, new_layout) }
    };

    let ptr = match (ptr, on_failure) {
        (Ok(ptr), _) => ptr,
        (Err(_), OnFailure::Abort) => handle_alloc_error(layout),
        (Err(_), OnFailure::Error) => return Err(AllocError),
    };

    // Creating a new Heap using the re-alloced pointer.
    // Replacing the existing heap and forgetting it so
    // that no drop code happens, avoiding the
    unsafe {
        let new = box_from_raw_parts_in(ptr.cast(), new_capacity, alloc);
        let old = std::mem::replace(b, new);
        std::mem::forget(old);
    }

    Ok(())
}
