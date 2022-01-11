//! The raw vector type that back-up the [`GenericVec`](crate::GenericVec)

use core::mem::MaybeUninit;
#[cfg(feature = "alloc")]
use std::boxed::Box;

mod array;
#[cfg(any(doc, feature = "alloc"))]
pub(crate) mod heap;
mod slice;

mod capacity;

/// Error on failure to allocate
pub struct AllocError;
/// Result of an allocation
pub type AllocResult = Result<(), AllocError>;

/// A type that can hold `Self::Item`s, and potentially reserve space for more.
///
/// # Safety
/// Other safe types rely on this trait being implemented correctly.
/// See the safety requirements on each function
pub unsafe trait Storage: AsRef<[MaybeUninit<Self::Item>]> + AsMut<[MaybeUninit<Self::Item>]> {
    /// The type of item that this storage can contain
    type Item;

    #[doc(hidden)]
    const CONST_CAPACITY: Option<usize> = None;

    /// Reserves space for at least `new_capacity` elements
    ///
    /// # Safety
    ///
    /// After this call successfully ends, the `capacity` must be at least
    /// `new_capacity`
    ///
    /// # Panic/Abort
    ///
    /// Maybe panic or abort if it is impossible to set the `capacity` to at
    /// least `new_capacity`
    fn reserve(&mut self, new_capacity: usize);

    /// Tries to reserve space for at least `new_capacity` elements
    ///
    /// # Safety
    /// If `Ok(())` is returned, the `capacity` must be at least `new_capacity`
    ///
    /// # Errors
    /// If enough space cannot be reserved, returns Err(AllocError)
    fn try_reserve(&mut self, new_capacity: usize) -> AllocResult;
}

/// A storage that can be initially created with a given capacity
///
/// # Safety
///
/// The storage must have a capacity of at least `capacity` after
/// `StorageWithCapacity::with_capacity` is called.
pub unsafe trait StorageWithCapacity: Storage + Sized {
    /// Creates a new storage with at least the given storage capacity
    fn with_capacity(capacity: usize) -> Self;

    #[doc(hidden)]
    #[allow(non_snake_case)]
    fn __with_capacity__const_capacity_checked(capacity: usize, _old_capacity: Option<usize>) -> Self {
        Self::with_capacity(capacity)
    }
}

unsafe impl<S: ?Sized + Storage> Storage for &mut S {
    type Item = S::Item;

    #[doc(hidden)]
    const CONST_CAPACITY: Option<usize> = S::CONST_CAPACITY;

    #[inline]
    fn reserve(&mut self, new_capacity: usize) { S::reserve(self, new_capacity); }
    #[inline]
    fn try_reserve(&mut self, new_capacity: usize) -> AllocResult { S::try_reserve(self, new_capacity) }
}

/// Wrapper for a [`Box<S>`]. Needed to implement some traits that could not be implemented on Box directly
#[cfg(any(doc, feature = "alloc"))]
pub struct BoxStorage<S: ?Sized + Storage>(pub Box<S>);

#[cfg(any(doc, feature = "alloc"))]
impl<S: ?Sized + Storage> AsRef<[MaybeUninit<S::Item>]> for BoxStorage<S> {
    fn as_ref(&self) -> &[MaybeUninit<S::Item>] { self.0.as_ref().as_ref() }
}

#[cfg(any(doc, feature = "alloc"))]
impl<S: ?Sized + Storage> AsMut<[MaybeUninit<S::Item>]> for BoxStorage<S> {
    fn as_mut(&mut self) -> &mut [MaybeUninit<S::Item>] { self.0.as_mut().as_mut() }
}

#[cfg(any(doc, feature = "alloc"))]
unsafe impl<S: ?Sized + Storage> Storage for BoxStorage<S> {
    type Item = S::Item;

    #[doc(hidden)]
    const CONST_CAPACITY: Option<usize> = S::CONST_CAPACITY;

    #[inline]
    fn reserve(&mut self, new_capacity: usize) { S::reserve(&mut self.0, new_capacity); }
    #[inline]
    fn try_reserve(&mut self, new_capacity: usize) -> AllocResult { S::try_reserve(&mut self.0, new_capacity) }
}

#[cfg(any(doc, feature = "alloc"))]
unsafe impl<S: ?Sized + StorageWithCapacity> StorageWithCapacity for BoxStorage<S> {
    fn with_capacity(capacity: usize) -> Self { Self(Box::new(S::with_capacity(capacity))) }

    #[doc(hidden)]
    #[allow(non_snake_case)]
    fn __with_capacity__const_capacity_checked(capacity: usize, old_capacity: Option<usize>) -> Self {
        Self(Box::new(S::__with_capacity__const_capacity_checked(
            capacity,
            old_capacity,
        )))
    }
}
