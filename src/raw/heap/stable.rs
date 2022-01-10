use crate::raw::{AllocError, AllocResult, Storage, StorageWithCapacity};

use core::alloc::Layout;
use std::{
    alloc::{alloc, handle_alloc_error, realloc},
    mem::MaybeUninit,
    ptr::NonNull,
};

type Heap<T> = Box<[MaybeUninit<T>]>;

enum OnFailure {
    Abort,
    Error,
}

/// Create a new `Heap<T>`storage from the given pointer and capacity
///
/// # Safety
///
/// If the capacity is non-zero
/// * You must have allocated the pointer from the global allocator
/// * The pointer must be valid to read-write for the range `ptr..ptr.add(capacity)`
pub(crate) unsafe fn box_from_raw_parts<T>(ptr: NonNull<T>, capacity: usize) -> Heap<T> {
    let ptr = std::ptr::slice_from_raw_parts_mut(ptr.as_ptr().cast(), capacity);
    Box::from_raw(ptr)
}

/// Convert a `Heap` storage into a pointer and capacity, without
/// deallocating the storage
pub(crate) fn box_into_raw_parts<T>(b: Heap<T>) -> (NonNull<T>, usize) {
    let ptr = Box::into_raw(b);
    unsafe {
        let capacity = (*ptr).len(); // probably not great but ptr_metadata is still nightly
        (NonNull::new_unchecked(ptr.cast()), capacity)
    }
}

unsafe impl<T> Storage for Heap<T> {
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

pub fn padding_needed_for(layout: Layout, align: usize) -> usize {
    let len = layout.size();

    // Rounded up value is:
    //   len_rounded_up = (len + align - 1) & !(align - 1);
    // and then we return the padding difference: `len_rounded_up - len`.
    //
    // We use modular arithmetic throughout:
    //
    // 1. align is guaranteed to be > 0, so align - 1 is always
    //    valid.
    //
    // 2. `len + align - 1` can overflow by at most `align - 1`,
    //    so the &-mask with `!(align - 1)` will ensure that in the
    //    case of overflow, `len_rounded_up` will itself be 0.
    //    Thus the returned padding, when added to `len`, yields 0,
    //    which trivially satisfies the alignment `align`.
    //
    // (Of course, attempts to allocate blocks of memory whose
    // size and padding overflow in the above manner should cause
    // the allocator to yield an error anyway.)

    let len_rounded_up = len.wrapping_add(align).wrapping_sub(1) & !align.wrapping_sub(1);
    len_rounded_up.wrapping_sub(len)
}

pub fn repeat(layout: Layout, n: usize) -> Result<Layout, ()> {
    // This cannot overflow. Quoting from the invariant of Layout:
    // > `size`, when rounded up to the nearest multiple of `align`,
    // > must not overflow (i.e., the rounded value must be less than
    // > `usize::MAX`)
    let padded_size = layout.size() + padding_needed_for(layout, layout.align());
    let alloc_size = padded_size.checked_mul(n).ok_or(())?;

    // SAFETY: self.align is already known to be valid and alloc_size has been
    // padded already.
    unsafe { Ok(Layout::from_size_align_unchecked(alloc_size, layout.align())) }
}

fn box_with_capacity<T>(capacity: usize) -> Heap<T> {
    if core::mem::size_of::<T>() == 0 {
        return Box::default()
    }

    let layout = repeat(Layout::new::<T>(), capacity).expect("Invalid layout");

    let ptr = unsafe { alloc(layout) };

    let ptr = match core::ptr::NonNull::new(ptr) {
        Some(ptr) => ptr,
        None => handle_alloc_error(layout),
    };

    // Safety:
    // we have allocated a pointer in global that has `capacity` elements available
    unsafe { box_from_raw_parts(ptr.cast(), capacity) }
}

unsafe impl<T> StorageWithCapacity for Heap<T> {
    fn with_capacity(cap: usize) -> Self { box_with_capacity(cap) }
}

#[cold]
#[inline(never)]
fn reserve_slow<T>(b: &mut Heap<T>, new_capacity: usize, on_failure: OnFailure) -> AllocResult {
    assert!(new_capacity > b.len());

    // taking a copy of the box so we can get it's contents and then update it later
    // Safety:
    // we forget the box just as soon we we copy it, so we have no risk of double-free
    let (ptr, cap) = unsafe { box_into_raw_parts(std::ptr::read(b)) };

    // grow by at least doubling
    let new_capacity = new_capacity
        .max(cap.checked_mul(2).expect("Could not grow further"))
        .max(super::INIT_ALLOC_CAPACITY);
    let layout = repeat(Layout::new::<T>(), new_capacity).expect("Invalid layout");

    let ptr = if cap == 0 {
        unsafe { alloc(layout) }
    } else {
        let new_layout = layout;
        let old_layout = repeat(Layout::new::<T>(), cap).expect("Invalid layout");

        unsafe { realloc(ptr.as_ptr().cast(), old_layout, new_layout.size()) }
    };

    let ptr = match (core::ptr::NonNull::new(ptr), on_failure) {
        (Some(ptr), _) => ptr,
        (None, OnFailure::Abort) => handle_alloc_error(layout),
        (None, OnFailure::Error) => return Err(AllocError),
    };

    // Creating a new Heap using the re-alloced pointer.
    // Replacing the existing heap and forgetting it so
    // that no drop code happens, avoiding the
    unsafe {
        let new = box_from_raw_parts(ptr.cast(), new_capacity);
        let old = std::mem::replace(b, new);
        std::mem::forget(old);
    }

    Ok(())
}
