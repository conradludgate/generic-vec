#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(feature = "nightly", feature(min_const_generics, unsafe_block_in_unsafe_fn))]
#![cfg_attr(
    feature = "nightly",
    feature(
        trusted_len,
        min_specialization,
        exact_size_is_empty,
        allocator_api,
        alloc_layout_extra
    )
)]
#![cfg_attr(feature = "nightly", forbid(unsafe_op_in_unsafe_fn))]
#![allow(unused_unsafe)]

#[cfg(all(feature = "alloc", not(feature = "std")))]
extern crate alloc as std;

use core::{
    marker::PhantomData,
    mem::MaybeUninit,
    ops::{Deref, DerefMut},
    ptr,
};

mod extension;
mod impls;
mod set_len;

pub mod iter;
pub mod raw;

use raw::RawVec;

#[cfg(feature = "alloc")]
#[cfg(feature = "nightly")]
pub type Vec<T, A = std::alloc::Global> = GenericVec<raw::Heap<T, A>>;
#[cfg(feature = "alloc")]
#[cfg(not(feature = "nightly"))]
pub type Vec<T> = GenericVec<raw::Heap<T>>;

#[cfg(feature = "nightly")]
pub type ArrayVec<T, const N: usize> = GenericVec<raw::UninitArray<T, N>>;
pub type SliceVec<'a, T> = GenericVec<raw::UninitSlice<'a, T>>;

#[cfg(feature = "nightly")]
pub type InitArrayVec<T, const N: usize> = GenericVec<raw::Array<T, N>>;
pub type InitSliceVec<'a, T> = GenericVec<raw::Slice<'a, T>>;

use iter::{Drain, DrainFilter, RawDrain, Splice};

#[doc(hidden)]
pub mod macros {
    pub use core::mem::MaybeUninit;
    impl<T> Uninit for T {}
    pub trait Uninit: Sized {
        const UNINIT: MaybeUninit<Self> = MaybeUninit::uninit();
    }
}

#[macro_export]
macro_rules! uninit_array {
    (const $n:expr) => {
        [$crate::macros::Uninit::UNINIT; $n]
    };

    ($n:expr) => {
        unsafe { $crate::macros::MaybeUninit::<[$crate::macros::MaybeUninit<_>; $n]>::uninit().assume_init() }
    };
}

#[repr(C)]
pub struct GenericVec<A: ?Sized + RawVec> {
    len: usize,
    mark: PhantomData<A::Item>,
    raw: A,
}

impl<A: ?Sized + RawVec> Deref for GenericVec<A> {
    type Target = [A::Item];

    fn deref(&self) -> &Self::Target {
        let len = self.len();
        unsafe { core::slice::from_raw_parts(self.as_ptr(), len) }
    }
}

impl<A: ?Sized + RawVec> DerefMut for GenericVec<A> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        let len = self.len();
        unsafe { core::slice::from_raw_parts_mut(self.as_mut_ptr(), len) }
    }
}

impl<A: ?Sized + RawVec> Drop for GenericVec<A> {
    fn drop(&mut self) { unsafe { ptr::drop_in_place(self.as_mut_slice()) } }
}

impl<A: RawVec> GenericVec<A> {
    pub fn with_raw(raw: A) -> Self {
        Self {
            raw,
            len: 0,
            mark: PhantomData,
        }
    }
}

impl<A: raw::RawVecWithCapacity> GenericVec<A> {
    pub fn with_capacity(capacity: usize) -> Self { Self::with_raw(A::with_capacity(capacity)) }

    #[inline]
    #[allow(non_snake_case)]
    fn __with_capacity__const_capacity_checked(capacity: usize, old_capacity: Option<usize>) -> Self {
        Self::with_raw(A::__with_capacity__const_capacity_checked(capacity, old_capacity))
    }
}

#[cfg(feature = "nightly")]
impl<T, const N: usize> ArrayVec<T, N> {
    pub const fn new() -> Self {
        Self {
            len: 0,
            mark: PhantomData,
            raw: raw::UninitArray::uninit(),
        }
    }
}

#[cfg(feature = "nightly")]
impl<T: Copy, const N: usize> InitArrayVec<T, N> {
    pub fn new(array: [T; N]) -> Self {
        Self {
            len: N,
            mark: PhantomData,
            raw: raw::Array::new(array),
        }
    }
}

#[cfg(feature = "alloc")]
impl<T> Vec<T> {
    pub const fn new() -> Self {
        Self {
            len: 0,
            mark: PhantomData,
            raw: raw::Heap::new(),
        }
    }
}

#[cfg(feature = "alloc")]
#[cfg(feature = "nightly")]
impl<T, A: std::alloc::AllocRef> Vec<T, A> {
    pub fn with_alloc(alloc: A) -> Self { Self::with_raw(raw::Heap::with_alloc(alloc)) }
}

impl<'a, T> SliceVec<'a, T> {
    pub fn new(slice: &'a mut [MaybeUninit<T>]) -> Self { Self::with_raw(raw::Uninit(slice)) }
}

impl<'a, T: Copy> InitSliceVec<'a, T> {
    pub fn new(slice: &'a mut [T]) -> Self {
        let len = slice.len();
        let mut vec = Self::with_raw(raw::Init(slice));
        vec.set_len(len);
        vec
    }
}

impl<A: ?Sized + RawVec> GenericVec<A> {
    pub fn as_ptr(&self) -> *const A::Item { self.raw.as_ptr() }

    pub fn as_mut_ptr(&mut self) -> *mut A::Item { self.raw.as_mut_ptr() }

    pub fn len(&self) -> usize { self.len }

    pub fn is_empty(&self) -> bool { self.len() == 0 }

    pub unsafe fn set_len_unchecked(&mut self, len: usize) { self.len = len; }

    pub fn set_len(&mut self, len: usize)
    where
        A: raw::RawVecInit,
    {
        unsafe {
            assert!(
                len <= self.capacity(),
                "Tried to set the length to larger than the capacity"
            );
            self.set_len_unchecked(len);
        }
    }

    pub fn capacity(&self) -> usize { self.raw.capacity() }

    pub fn as_slice(&self) -> &[A::Item] { self }

    pub fn as_mut_slice(&mut self) -> &mut [A::Item] { self }

    pub unsafe fn raw_buffer(&self) -> &A { &self.raw }

    pub unsafe fn raw_buffer_mut(&mut self) -> &mut A { &mut self.raw }

    pub fn remaining(&mut self) -> &mut [A::BufferItem] {
        unsafe {
            let len = self.len();
            let cap = self.raw.capacity();
            core::slice::from_raw_parts_mut(self.raw.as_mut_ptr().add(len).cast(), cap.wrapping_sub(len))
        }
    }

    pub fn reserve(&mut self, additional: usize) {
        if let Some(new_capacity) = self.len().checked_add(additional) {
            self.raw.reserve(new_capacity)
        }
    }

    pub fn try_reserve(&mut self, additional: usize) -> Result<(), raw::AllocError> {
        if let Some(new_capacity) = self.len().checked_add(additional) {
            self.raw.try_reserve(new_capacity)
        } else {
            Ok(())
        }
    }

    pub fn truncate(&mut self, len: usize) {
        if let Some(diff) = self.len().checked_sub(len) {
            unsafe {
                self.set_len_unchecked(len);
                core::slice::from_raw_parts_mut(self.as_mut_ptr().add(len), diff);
            }
        }
    }

    pub fn grow(&mut self, additional: usize, value: A::Item)
    where
        A::Item: Clone,
    {
        self.reserve(additional);
        unsafe { extension::Extension::grow(self, additional, value) }
    }

    pub fn clear(&mut self) { self.truncate(0); }

    pub fn push(&mut self, value: A::Item) -> &mut A::Item {
        if self.len() == self.capacity() {
            self.reserve(1);
        }

        unsafe { self.push_unchecked(value) }
    }

    #[cfg(feature = "nightly")]
    pub fn push_array<const N: usize>(&mut self, value: [A::Item; N]) -> &mut [A::Item; N] {
        if self.capacity().wrapping_sub(self.len()) < N {
            self.reserve(N);
        }

        unsafe { self.push_array_unchecked(value) }
    }

    pub fn insert(&mut self, index: usize, value: A::Item) -> &mut A::Item {
        assert!(
            index <= self.len(),
            "Tried to insert at {}, but length is {}",
            index,
            self.len(),
        );

        if self.len() == self.capacity() {
            self.reserve(1);
        }

        unsafe { self.insert_unchecked(index, value) }
    }

    #[cfg(feature = "nightly")]
    pub fn insert_array<const N: usize>(&mut self, index: usize, value: [A::Item; N]) -> &mut [A::Item; N] {
        assert!(
            index <= self.len(),
            "Tried to insert at {}, but length is {}",
            index,
            self.len(),
        );

        if self.capacity().wrapping_sub(self.len()) < N {
            self.reserve(N);
        }

        unsafe { self.insert_array_unchecked(index, value) }
    }

    pub fn pop(&mut self) -> A::Item {
        assert_ne!(self.len(), 0, "Tried to pop an element from an empty vector",);

        unsafe { self.pop_unchecked() }
    }

    #[cfg(feature = "nightly")]
    pub fn pop_array<const N: usize>(&mut self) -> [A::Item; N] {
        assert_ne!(
            self.len(),
            0,
            "Tried to pop {} elements, but length is {}",
            N,
            self.len()
        );

        unsafe { self.pop_array_unchecked() }
    }

    pub fn remove(&mut self, index: usize) -> A::Item {
        assert!(
            index < self.len(),
            "Tried to remove item at index {}, but length is {}",
            index,
            self.len()
        );

        unsafe { self.remove_unchecked(index) }
    }

    #[cfg(feature = "nightly")]
    pub fn remove_array<const N: usize>(&mut self, index: usize) -> [A::Item; N] {
        assert!(
            self.len() >= index && self.len().wrapping_sub(index) >= N,
            "Tried to remove {} elements at index {}, but length is {}",
            N,
            index,
            self.len()
        );

        unsafe { self.remove_array_unchecked(index) }
    }

    pub fn swap_remove(&mut self, index: usize) -> A::Item {
        assert!(
            index < self.len(),
            "Tried to remove item at index {}, but length is {}",
            index,
            self.len()
        );

        unsafe { self.swap_remove_unchecked(index) }
    }

    pub fn try_push(&mut self, value: A::Item) -> Result<&mut A::Item, A::Item> {
        if self.len() == self.capacity() {
            Err(value)
        } else {
            unsafe { Ok(self.push_unchecked(value)) }
        }
    }

    #[cfg(feature = "nightly")]
    pub fn try_push_array<const N: usize>(&mut self, value: [A::Item; N]) -> Result<&mut [A::Item; N], [A::Item; N]> {
        if self.capacity().wrapping_sub(self.len()) < N {
            Err(value)
        } else {
            unsafe { Ok(self.push_array_unchecked(value)) }
        }
    }

    pub fn try_insert(&mut self, index: usize, value: A::Item) -> Result<&mut A::Item, A::Item> {
        if self.len() == self.capacity() || index > self.len() {
            Err(value)
        } else {
            unsafe { Ok(self.insert_unchecked(index, value)) }
        }
    }

    #[cfg(feature = "nightly")]
    pub fn try_insert_array<const N: usize>(
        &mut self,
        index: usize,
        value: [A::Item; N],
    ) -> Result<&mut [A::Item; N], [A::Item; N]> {
        if self.capacity().wrapping_sub(self.len()) < N || index > self.len() {
            Err(value)
        } else {
            unsafe { Ok(self.insert_array_unchecked(index, value)) }
        }
    }

    pub fn try_pop(&mut self) -> Option<A::Item> {
        if self.len() == 0 {
            None
        } else {
            unsafe { Some(self.pop_unchecked()) }
        }
    }

    #[cfg(feature = "nightly")]
    pub fn try_pop_array<const N: usize>(&mut self) -> Option<[A::Item; N]> {
        if self.len() == 0 {
            None
        } else {
            unsafe { Some(self.pop_array_unchecked()) }
        }
    }

    pub fn try_remove(&mut self, index: usize) -> Option<A::Item> {
        if self.len() < index {
            None
        } else {
            unsafe { Some(self.remove_unchecked(index)) }
        }
    }

    #[cfg(feature = "nightly")]
    pub fn try_remove_array<const N: usize>(&mut self, index: usize) -> Option<[A::Item; N]> {
        if self.len() < index || self.len().wrapping_sub(index) < N {
            unsafe { Some(self.remove_array_unchecked(index)) }
        } else {
            None
        }
    }

    pub fn try_swap_remove(&mut self, index: usize) -> Option<A::Item> {
        if index < self.len() {
            unsafe { Some(self.swap_remove_unchecked(index)) }
        } else {
            None
        }
    }

    pub unsafe fn push_unchecked(&mut self, value: A::Item) -> &mut A::Item {
        match A::CONST_CAPACITY {
            Some(0) => panic!("Tried to push an element into a zero-capacity vector!"),
            _ => (),
        }

        debug_assert_ne!(
            self.len(),
            self.capacity(),
            "Tried to `push_unchecked` past capacity! This is UB in release mode"
        );
        unsafe {
            let len = self.len();
            self.set_len_unchecked(len.wrapping_add(1));
            let ptr = self.as_mut_ptr().add(len);
            ptr.write(value);
            &mut *ptr
        }
    }

    #[cfg(feature = "nightly")]
    pub unsafe fn push_array_unchecked<const N: usize>(&mut self, value: [A::Item; N]) -> &mut [A::Item; N] {
        match A::CONST_CAPACITY {
            Some(n) if n < N => {
                panic!("Tried to push an array larger than the maximum capacity of the vector!")
            }
            _ => (),
        }

        unsafe {
            let len = self.len();
            self.set_len_unchecked(len.wrapping_add(N));
            let ptr = self.as_mut_ptr();
            let out = ptr.add(len) as *mut [A::Item; N];
            out.write(value);
            &mut *out
        }
    }

    pub unsafe fn insert_unchecked(&mut self, index: usize, value: A::Item) -> &mut A::Item {
        unsafe {
            match A::CONST_CAPACITY {
                Some(0) => panic!("Tried to insert an element into a zero-capacity vector!"),
                _ => (),
            }

            let len = self.len();
            self.set_len_unchecked(len.wrapping_add(1));
            let ptr = self.raw.as_mut_ptr().add(index);
            ptr.add(1).copy_from(ptr, len.wrapping_sub(index));
            ptr.write(value);
            &mut *ptr
        }
    }

    #[cfg(feature = "nightly")]
    pub unsafe fn insert_array_unchecked<const N: usize>(
        &mut self,
        index: usize,
        value: [A::Item; N],
    ) -> &mut [A::Item; N] {
        match A::CONST_CAPACITY {
            Some(n) if n < N => {
                panic!("Tried to push an array larger than the maximum capacity of the vector!")
            }
            _ => (),
        }

        unsafe {
            let len = self.len();
            self.set_len_unchecked(len.wrapping_add(N));
            let ptr = self.as_mut_ptr();
            let dist = len.wrapping_sub(index);

            let out = ptr.add(index);
            out.add(N).copy_from(out, dist);
            let out = out as *mut [A::Item; N];
            out.write(value);
            &mut *out
        }
    }

    pub unsafe fn pop_unchecked(&mut self) -> A::Item {
        match A::CONST_CAPACITY {
            Some(0) => panic!("Tried to remove an element from a zero-capacity vector!"),
            _ => (),
        }

        let len = self.len();
        debug_assert_ne!(
            len, 0,
            "Tried to `pop_unchecked` an empty array vec! This is UB in release mode"
        );
        unsafe {
            let len = len.wrapping_sub(1);
            self.set_len_unchecked(len);
            self.as_mut_ptr().add(len).read()
        }
    }

    #[cfg(feature = "nightly")]
    pub unsafe fn pop_array_unchecked<const N: usize>(&mut self) -> [A::Item; N] {
        match A::CONST_CAPACITY {
            Some(n) if n < N => panic!("Tried to remove {} elements from a {} capacity vector!", N, n),
            _ => (),
        }

        let len = self.len();
        debug_assert!(
            len > N,
            "Tried to remove {} elements from a {} length vector! This is UB in release mode",
            N,
            len,
        );
        unsafe {
            let len = len.wrapping_sub(N);
            self.set_len_unchecked(len);
            self.as_mut_ptr().add(len).cast::<[A::Item; N]>().read()
        }
    }

    pub unsafe fn remove_unchecked(&mut self, index: usize) -> A::Item {
        unsafe {
            match A::CONST_CAPACITY {
                Some(0) => panic!("Tried to remove an element from a zero-capacity vector!"),
                _ => (),
            }

            let len = self.len();

            debug_assert!(
                index <= len,
                "Tried to remove an element at index {} from a {} length vector! This is UB in release mode",
                index,
                len,
            );

            self.set_len_unchecked(len.wrapping_sub(1));
            let ptr = self.raw.as_mut_ptr().add(index);
            let value = ptr.read();
            ptr.copy_from(ptr.add(1), len.wrapping_sub(index).wrapping_sub(1));
            value
        }
    }

    #[cfg(feature = "nightly")]
    pub unsafe fn remove_array_unchecked<const N: usize>(&mut self, index: usize) -> [A::Item; N] {
        match A::CONST_CAPACITY {
            Some(n) if n < N => panic!("Tried to remove {} elements from a {} capacity vector!", N, n),
            _ => (),
        }

        let len = self.len();
        debug_assert!(
            index <= len,
            "Tried to remove elements at index {} from a {} length vector! This is UB in release mode",
            index,
            len,
        );
        debug_assert!(
            len.wrapping_sub(index) > N,
            "Tried to remove {} elements from a {} length vector! This is UB in release mode",
            N,
            len,
        );
        unsafe {
            self.set_len_unchecked(len.wrapping_sub(N));
            let ptr = self.as_mut_ptr().add(index);
            let value = ptr.cast::<[A::Item; N]>().read();
            if N != 0 {
                ptr.copy_from(ptr.add(N), len.wrapping_sub(index).wrapping_sub(N));
            }
            value
        }
    }

    pub unsafe fn swap_remove_unchecked(&mut self, index: usize) -> A::Item {
        unsafe {
            match A::CONST_CAPACITY {
                Some(0) => panic!("Tried to remove an element from a zero-capacity vector!"),
                _ => (),
            }

            let len = self.len();
            self.set_len_unchecked(len.wrapping_sub(1));
            let ptr = self.raw.as_mut_ptr();
            let at = ptr.add(index);
            let end = ptr.add(len.wrapping_sub(1));
            let value = at.read();
            at.copy_from(end, 1);
            value
        }
    }

    pub fn split_off<B>(&mut self, index: usize) -> GenericVec<B>
    where
        B: raw::RawVecWithCapacity<Item = A::Item>,
    {
        assert!(
            index <= self.len(),
            "Tried to split at index {}, but length is {}",
            index,
            self.len()
        );

        let mut vec =
            GenericVec::<B>::__with_capacity__const_capacity_checked(self.len().wrapping_sub(index), A::CONST_CAPACITY);

        self.split_off_into(index, &mut vec);

        vec
    }

    pub fn split_off_into<B>(&mut self, index: usize, other: &mut GenericVec<B>)
    where
        B: raw::RawVec<Item = A::Item>,
    {
        assert!(
            index <= self.len(),
            "Tried to split at index {}, but length is {}",
            index,
            self.len()
        );

        unsafe {
            let slice = self.get_unchecked(index..);
            other.reserve(slice.len());
            other.extend_from_slice_unchecked(slice);
            self.set_len_unchecked(index);
        }
    }

    pub fn convert<B: raw::RawVecWithCapacity<Item = A::Item>>(mut self) -> GenericVec<B>
    where
        A: Sized,
    {
        self.split_off(0)
    }

    pub fn consume_extend<B: raw::RawVec<Item = A::Item>>(&mut self, other: &mut GenericVec<B>) {
        unsafe {
            self.reserve(other.len());
            self.extend_from_slice_unchecked(other);
            other.set_len_unchecked(0);
        }
    }

    #[inline]
    pub fn raw_drain<R>(&mut self, range: R) -> RawDrain<'_, A>
    where
        R: core::slice::SliceIndex<[A::Item], Output = [A::Item]>,
    {
        RawDrain::new(self, range)
    }

    #[inline]
    pub fn drain<R>(&mut self, range: R) -> Drain<'_, A>
    where
        R: core::slice::SliceIndex<[A::Item], Output = [A::Item]>,
    {
        self.raw_drain(range).into()
    }

    #[inline]
    pub fn drain_filter<R, F>(&mut self, range: R, f: F) -> DrainFilter<'_, A, F>
    where
        R: core::slice::SliceIndex<[A::Item], Output = [A::Item]>,
        F: FnMut(&mut A::Item) -> bool,
    {
        DrainFilter::new(self.raw_drain(range), f)
    }

    #[inline]
    pub fn splice<R, I>(&mut self, range: R, replace_with: I) -> Splice<'_, A, I::IntoIter>
    where
        R: core::slice::SliceIndex<[A::Item], Output = [A::Item]>,
        I: IntoIterator<Item = A::Item>,
    {
        Splice::new(self.raw_drain(range), replace_with.into_iter())
    }

    #[inline]
    pub fn retain<F>(&mut self, f: F)
    where
        F: FnMut(&mut A::Item) -> bool,
    {
        fn not<F: FnMut(&mut T) -> bool, T>(mut f: F) -> impl FnMut(&mut T) -> bool { move |value| !f(value) }
        self.drain_filter(.., not(f));
    }

    pub unsafe fn extend_from_slice_unchecked(&mut self, slice: &[A::Item]) {
        unsafe {
            let len = self.len();
            self.as_mut_ptr()
                .add(len)
                .copy_from_nonoverlapping(slice.as_ptr(), slice.len());
            self.set_len_unchecked(len.wrapping_add(slice.len()));
        }
    }

    pub fn extend_from_slice(&mut self, slice: &[A::Item])
    where
        A::Item: Clone,
    {
        self.reserve(self.len());

        unsafe { extension::Extension::extend_from_slice(self, slice) }
    }
}
