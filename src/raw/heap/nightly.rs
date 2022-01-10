use crate::raw::{
    capacity::{capacity, Round},
    AllocError, AllocResult, Storage, StorageWithCapacity,
};

use core::{alloc::Layout, mem::size_of, ptr::NonNull};
use std::{alloc::handle_alloc_error, mem::MaybeUninit};

use std::alloc::{Allocator, Global};

doc_heap! {
    #[repr(C)]
    #[cfg_attr(doc, doc(cfg(feature = "alloc")))]
    ///
    /// The allocator type paramter is only available on `nightly`
    pub struct Heap<T, A: Allocator = Global>(Box<[MaybeUninit<T>], A>);
}

unsafe impl<T, A: Allocator + Send> Send for Heap<T, A> {}
unsafe impl<T, A: Allocator + Sync> Sync for Heap<T, A> {}

enum OnFailure {
    Abort,
    Error,
}

impl<T> Heap<T> {
    /// Create a new zero-capacity heap vector
    pub fn new() -> Self { Self(Box::new_uninit_slice(0)) }

    /// Create a new `Heap<T>`storage from the given pointer and capacity
    ///
    /// # Safety
    ///
    /// If the capacity is non-zero
    /// * You must have allocated the pointer from the [`Global`] allocator
    /// * The pointer must be valid to read-write for the range `ptr..ptr.add(capacity)`
    pub unsafe fn from_raw_parts(ptr: NonNull<T>, capacity: usize) -> Self {
        unsafe { Self::from_raw_parts_in(ptr, capacity, Global) }
    }

    /// Convert a `Heap` storage into a pointer and capacity, without
    /// deallocating the storage
    pub fn into_raw_parts(self) -> (NonNull<T>, usize) {
        let ptr = Box::into_raw(self.0);
        unsafe {
            let (ptr, capacity) = ptr.to_raw_parts();
            (NonNull::new_unchecked(ptr.cast()), capacity)
        }
    }
}

#[cfg_attr(doc, doc(cfg(feature = "nightly")))]
impl<T, A: Allocator> Heap<T, A> {
    /// Create a new zero-capacity heap vector with the given allocator
    pub fn with_alloc(allocator: A) -> Self { Self(Box::new_uninit_slice_in(0, allocator)) }

    /// Create a new `Heap<T>`storage from the given pointer and capacity
    ///
    /// # Safety
    ///
    /// If the capacity is non-zero
    /// * You must have allocated the pointer from the given allocator
    /// * The pointer must be valid to read-write for the range `ptr..ptr.add(capacity)`
    pub unsafe fn from_raw_parts_in(ptr: NonNull<T>, capacity: usize, allocator: A) -> Self {
        unsafe {
            let ptr = std::ptr::slice_from_raw_parts_mut(ptr.as_ptr().cast(), capacity);
            Self(Box::from_raw_in(ptr, allocator))
        }
    }

    /// Convert a `Heap` storage into a pointer and capacity, without
    /// deallocating the storage
    pub fn into_raw_parts_with_alloc(self) -> (NonNull<T>, usize, A) {
        let (ptr, alloc) = Box::into_raw_with_allocator(self.0);
        unsafe {
            let (ptr, capacity) = ptr.to_raw_parts();
            (NonNull::new_unchecked(ptr.cast()), capacity, alloc)
        }
    }
}

impl<T, A: Allocator + Default> Default for Heap<T, A> {
    fn default() -> Self { Self::with_alloc(Default::default()) }
}

impl<T, A: Allocator> AsRef<[MaybeUninit<T>]> for Heap<T, A> {
    fn as_ref(&self) -> &[MaybeUninit<T>] { self.0.as_ref() }
}

impl<T, A: Allocator> AsMut<[MaybeUninit<T>]> for Heap<T, A> {
    fn as_mut(&mut self) -> &mut [MaybeUninit<T>] { self.0.as_mut() }
}

unsafe impl<T, A: Allocator> Storage for Heap<T, A> {
    type Item = T;

    fn reserve(&mut self, new_capacity: usize) {
        let new_capacity = capacity(new_capacity, size_of::<T>(), size_of::<T>(), Round::Up);
        if self.0.len() < new_capacity {
            let _ = self.reserve_slow(new_capacity, OnFailure::Abort);
        }
    }

    fn try_reserve(&mut self, new_capacity: usize) -> AllocResult {
        let new_capacity = capacity(new_capacity, size_of::<T>(), size_of::<T>(), Round::Up);
        if self.0.len() < new_capacity {
            self.reserve_slow(new_capacity, OnFailure::Error)
        } else {
            Ok(())
        }
    }
}

impl<T, A: Default + Allocator> Heap<T, A> {
    fn with_capacity(capacity: usize) -> Self { Self(Box::new_uninit_slice_in(capacity, A::default())) }
}

unsafe impl<T, A: Default + Allocator> StorageWithCapacity for Heap<T, A> {
    fn with_capacity(cap: usize) -> Self {
        Self::with_capacity(capacity(cap, size_of::<T>(), size_of::<T>(), Round::Up))
    }
}

impl<T, A: Allocator> Heap<T, A> {
    #[cold]
    #[inline(never)]
    fn reserve_slow(&mut self, new_capacity: usize, on_failure: OnFailure) -> AllocResult {
        assert!(new_capacity > self.0.len());

        // taking a copy of the box so we can get it's contents and then update it later
        // Safety:
        // we forget the box just as soon we we copy it, so we have no risk of double-free
        let (ptr, cap, alloc) = unsafe { Self::into_raw_parts_with_alloc(std::ptr::read(self)) };

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
            let new = Self::from_raw_parts_in(ptr.cast(), new_capacity, alloc);
            let old = std::mem::replace(self, new);
            std::mem::forget(old);
        }

        Ok(())
    }
}
