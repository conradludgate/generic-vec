use crate::{raw::StorageWithCapacity, GenericVec, Storage};

#[allow(unused_imports)]
use core::{
    borrow::{Borrow, BorrowMut},
    hash::{Hash, Hasher},
    ops::{Index, IndexMut},
    ptr::NonNull,
    slice::SliceIndex,
};

#[cfg(feature = "alloc")]
use std::vec::Vec;

impl<S: StorageWithCapacity> Clone for GenericVec<S>
where
    S::Item: Clone,
{
    fn clone(&self) -> Self {
        let mut vec = Self::with_capacity(self.len());
        vec.extend_from_slice(self);
        vec
    }

    fn clone_from(&mut self, source: &Self) { self.clone_from(source); }
}

impl<S: StorageWithCapacity + Default> Default for GenericVec<S> {
    fn default() -> Self { Self::with_storage(Default::default()) }
}

impl<O: ?Sized + AsRef<[S::Item]>, S: ?Sized + Storage> PartialEq<O> for GenericVec<S>
where
    S::Item: PartialEq,
{
    fn eq(&self, other: &O) -> bool { self.as_slice() == other.as_ref() }
}

impl<S: ?Sized + Storage> Eq for GenericVec<S> where S::Item: Eq {}

impl<O: ?Sized + AsRef<[S::Item]>, S: ?Sized + Storage> PartialOrd<O> for GenericVec<S>
where
    S::Item: PartialOrd,
{
    fn partial_cmp(&self, other: &O) -> Option<core::cmp::Ordering> { self.as_slice().partial_cmp(other.as_ref()) }
}

impl<S: ?Sized + Storage> Ord for GenericVec<S>
where
    S::Item: Ord,
{
    fn cmp(&self, other: &Self) -> core::cmp::Ordering { self.as_slice().cmp(other.as_ref()) }
}

impl<S: ?Sized + Storage> Hash for GenericVec<S>
where
    S::Item: Hash,
{
    fn hash<H: Hasher>(&self, state: &mut H) { self.as_slice().hash(state) }
}

use core::fmt;
impl<S: ?Sized + Storage> fmt::Debug for GenericVec<S>
where
    S::Item: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { self.as_slice().fmt(f) }
}

impl<S: ?Sized + Storage> AsRef<[S::Item]> for GenericVec<S> {
    fn as_ref(&self) -> &[S::Item] { self }
}

impl<S: ?Sized + Storage> AsMut<[S::Item]> for GenericVec<S> {
    fn as_mut(&mut self) -> &mut [S::Item] { self }
}

impl<S: ?Sized + Storage> Borrow<[S::Item]> for GenericVec<S> {
    fn borrow(&self) -> &[S::Item] { self }
}

impl<S: ?Sized + Storage> BorrowMut<[S::Item]> for GenericVec<S> {
    fn borrow_mut(&mut self) -> &mut [S::Item] { self }
}

#[cfg(any(doc, feature = "nightly"))]
impl<T, const N: usize> From<[T; N]> for crate::ArrayVec<T, N> {
    fn from(array: [T; N]) -> Self { Self::from_array(array) }
}

#[cfg(any(doc, feature = "nightly"))]
impl<T, const N: usize> TryFrom<crate::ArrayVec<T, N>> for [T; N] {
    type Error = crate::ArrayVec<T, N>;

    fn try_from(value: crate::ArrayVec<T, N>) -> Result<Self, Self::Error> {
        value.try_into_array()
    }
}

#[cfg(not(doc))]
#[cfg(feature = "alloc")]
#[cfg(not(feature = "nightly"))]
impl<T> From<Vec<T>> for crate::HeapVec<T> {
    fn from(vec: Vec<T>) -> Self {
        let mut vec = core::mem::ManuallyDrop::new(vec);

        let len = vec.len();
        let cap = vec.capacity();
        let ptr = unsafe { NonNull::new_unchecked(vec.as_mut_ptr()) };

        unsafe { crate::HeapVec::from_raw_parts(len, crate::raw::Heap::from_raw_parts(ptr, cap)) }
    }
}

#[cfg(any(doc, feature = "alloc"))]
#[cfg(any(doc, feature = "nightly"))]
impl<T, A: std::alloc::Allocator> From<Vec<T, A>> for crate::HeapVec<T, A> {
    fn from(vec: Vec<T, A>) -> Self {
        let (ptr, len, cap, alloc) = vec.into_raw_parts_with_alloc();

        unsafe {
            crate::HeapVec::from_raw_parts(
                len,
                crate::raw::Heap::from_raw_parts_in(NonNull::new_unchecked(ptr), cap, alloc),
            )
        }
    }
}

#[cfg(not(doc))]
#[cfg(feature = "alloc")]
#[cfg(not(feature = "nightly"))]
impl<T> From<crate::HeapVec<T>> for Vec<T> {
    fn from(vec: crate::HeapVec<T>) -> Self {
        let (length, alloc) = vec.into_raw_parts();
        let (ptr, capacity) = alloc.into_raw_parts();

        unsafe { Vec::from_raw_parts(ptr.as_ptr(), length, capacity) }
    }
}

#[cfg(any(doc, feature = "alloc"))]
#[cfg(any(doc, feature = "nightly"))]
impl<T, A: std::alloc::Allocator> From<crate::HeapVec<T, A>> for Vec<T, A> {
    fn from(vec: crate::HeapVec<T, A>) -> Self {
        let (length, alloc) = vec.into_raw_parts();
        let (ptr, capacity, alloc) = alloc.into_raw_parts_with_alloc();

        unsafe { Vec::from_raw_parts_in(ptr.as_ptr(), length, capacity, alloc) }
    }
}

impl<S: Storage + ?Sized, I> Index<I> for GenericVec<S>
where
    I: SliceIndex<[S::Item]>,
{
    type Output = I::Output;

    fn index(&self, index: I) -> &Self::Output { self.as_slice().index(index) }
}

impl<S: Storage + ?Sized, I> IndexMut<I> for GenericVec<S>
where
    I: SliceIndex<[S::Item]>,
{
    fn index_mut(&mut self, index: I) -> &mut Self::Output { self.as_mut_slice().index_mut(index) }
}
