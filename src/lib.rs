#![cfg_attr(not(any(doc, feature = "std")), no_std)]
#![cfg_attr(
    any(doc, feature = "nightly"),
    feature(
        trusted_len,
        min_specialization,
        exact_size_is_empty,
        allocator_api,
        alloc_layout_extra,
        const_fn_trait_bound,
        const_mut_refs,
        doc_cfg,
        new_uninit,
        ptr_metadata,
    )
)]
#![cfg_attr(feature = "nightly", forbid(unsafe_op_in_unsafe_fn))]
#![allow(unused_unsafe)]
#![forbid(missing_docs, clippy::missing_safety_doc)]
#![warn(clippy::pedantic)]
#![allow(clippy::must_use_candidate, clippy::module_name_repetitions)]

//! A vector that can store items anywhere: in slices, arrays, or the heap!
//!
//! [`GenericVec`] has complete parity with [`Vec`], and even provides some features
//! that are only in `nightly` on `std` (like [`GenericVec::drain_filter`]), or a more permissive
//! interface like [`GenericVec::retain`]. In fact, you can trivially convert a [`Vec`] to a
//! [`HeapVec`] and back!
//!
//! This crate is `no_std` compatible, just turn off all default features.
//!
//! # Features
//!
//! * `std` (default) - enables you to use an allocator, and
//! * `alloc` - enables you to use an allocator, for heap allocated storages
//!     (like [`Vec`])
//! * `nightly` - enables you to use the Allocator trait
//!
//! # Basic Usage
//!
//! ### [`SliceVec`]
//!
//! [`SliceVec`] stores an uninit slice buffer, and they store all of thier values in that buffer.
//!
//! ```rust
//! use cl_generic_vec::{SliceVec, uninit_array};
//!
//! let mut uninit_buffer = uninit_array::<_, 16>();
//! let mut slice_vec = SliceVec::new(&mut uninit_buffer);
//!
//! assert!(slice_vec.is_empty());
//! slice_vec.push(10);
//! assert_eq!(slice_vec, [10]);
//! ```
//!
//! Of course if you try to push past a `*SliceVec`'s capacity
//! (the length of the slice you passed in), then it will panic.
//!
//! ### [`ArrayVec`](type@ArrayVec)
//!
//! [`ArrayVec`](type@ArrayVec) is like the slice version, but since they own their data,
//! they can be freely moved around, unconstrained. You can also create
//! a new [`ArrayVec`](type@ArrayVec) without passing in an existing buffer,
//! unlike the slice versions.
//!
//! ```rust
//! use cl_generic_vec::ArrayVec;
//!
//! let mut array_vec = ArrayVec::<i32, 16>::new();
//!
//! array_vec.push(10);
//! array_vec.push(20);
//! array_vec.push(30);
//!
//! assert_eq!(array_vec, [10, 20, 30]);
//! ```
//!
//! ## `alloc`
//!
//! A [`HeapVec`] is just [`Vec`], but built atop [`GenericVec`],
//! meaning you get all the features of [`GenericVec`] for free! But this
//! requries either the `alloc` or `std` feature to be enabled.
//!
//! ```rust
//! use cl_generic_vec::{HeapVec, gvec};
//! let mut vec: HeapVec<u32> = gvec![1, 2, 3, 4];
//! assert_eq!(vec.capacity(), 4);
//! vec.extend([5, 6, 7, 8]);
//!
//! assert_eq!(vec, [1, 2, 3, 4, 5, 6, 7, 8]);
//!
//! vec.try_push(5).expect_err("Tried to push past capacity!");
//! ```
//!
//! ## `nightly`
//!
//! On `nightly`
//! * a number of optimizations are enabled
//! * some diagnostics become better
//!
//! Note on the documentation: if the feature exists on [`Vec`], then the documentation
//! is either exactly the same as [`Vec`] or slightly adapted to better fit [`GenericVec`]
//!
//! Note on implementation: large parts of the implementation came straight from [`Vec`]
//! so thanks for the amazing reference `std`!

#[cfg(all(feature = "alloc", not(feature = "std")))]
extern crate alloc as std;

use core::{
    mem::MaybeUninit,
    ops::{Deref, DerefMut, RangeBounds},
    ptr,
};
use std::mem::ManuallyDrop;

mod extension;
mod impls;
mod slice;

pub mod iter;
pub mod raw;

use raw::{AllocError, AllocResult, Storage};

#[doc(hidden)]
pub use core;

/// A heap backed vector with a growable capacity
#[cfg(any(doc, all(feature = "alloc", feature = "nightly")))]
#[cfg_attr(doc, doc(cfg(all(feature = "alloc", feature = "nightly"))))]
pub type HeapVec<T, A = std::alloc::Global> = GenericVec<Box<[MaybeUninit<T>], A>>;

/// A heap backed vector with a growable capacity
#[cfg(all(not(doc), feature = "alloc", not(feature = "nightly")))]
#[cfg_attr(doc, doc(cfg(feature = "alloc")))]
pub type HeapVec<T> = GenericVec<Box<[MaybeUninit<T>]>>;

/// An array backed vector backed by potentially uninitialized memory
pub type ArrayVec<T, const N: usize> = GenericVec<[MaybeUninit<T>; N]>;
/// An slice backed vector backed by potentially uninitialized memory
pub type SliceVec<'a, T> = GenericVec<&'a mut [MaybeUninit<T>]>;

/// Creates a new uninit array, See [`MaybeUninit::uninit_array`]
pub fn uninit_array<T, const N: usize>() -> [MaybeUninit<T>; N] {
    unsafe { MaybeUninit::<[MaybeUninit<T>; N]>::uninit().assume_init() }
}

/// Create a new generic vector
///
/// Because this can create any generic vector, you will likely
/// need to add some type annotations when you use it,
///
/// ```rust
/// # use cl_generic_vec::{gvec, ArrayVec};
/// let x: ArrayVec<i32, 2> = gvec![0, 1];
/// assert_eq!(x, [0, 1]);
/// ```
#[macro_export]
#[cfg(feature = "nightly")]
macro_rules! gvec {
    ($expr:expr; $n:expr) => {{
        let len = $n;
        let mut vec = $crate::GenericVec::with_capacity(len);
        vec.grow(len, $expr);
        vec
    }};
    ($($expr:expr),*) => {{
        let expr = [$($expr),*];
        let mut vec = $crate::GenericVec::with_capacity(expr.len());
        unsafe { vec.push_array_unchecked(expr); }
        vec
    }};
}

#[doc(hidden)]
#[macro_export]
macro_rules! count {
    () => { 0 };
    ($($a:tt $b:tt)*) => { $crate::count!($($a)*) << 1 };
    ($c:tt $($a:tt $b:tt)*) => { ($crate::count!($($a)*) << 1) | 1 };
}

/// Create a new generic vector
///
/// Because this can create any generic vector, you will likely
/// need to add some type annotations when you use it,
///
/// ```rust
/// # use cl_generic_vec::{gvec, ArrayVec};
/// let x: ArrayVec<i32, 4> = gvec![1, 2, 3, 4];
/// assert_eq!(x, [1, 2, 3, 4]);
/// ```
#[macro_export]
#[cfg(not(feature = "nightly"))]
macro_rules! gvec {
    ($expr:expr; $n:expr) => {{
        let len = $n;
        let mut vec = $crate::GenericVec::with_capacity(len);
        vec.grow(len, $expr);
        vec
    }};
    ($($expr:expr),*) => {{
        let mut vec = $crate::GenericVec::with_capacity($crate::count!($(($expr))*));
        unsafe {
            $(vec.push_unchecked($expr);)*
        }
        vec
    }};
}

/// Save the changes to [`GenericVec::spare_capacity_mut`]
///
/// $orig - a mutable reference to a [`GenericVec`]
/// $spare - the [`SliceVec`] that was obtained from [`$orig.spare_capacity_mut()`]
///
/// # Safety
///
/// `$spare` should be the [`SliceVec`] returned by `$orig.spare_capacity_mut()`
#[macro_export]
macro_rules! save_spare {
    ($spare:expr, $orig:expr) => {{
        let spare: $crate::SliceVec<_> = $spare;
        let spare = $crate::core::mem::ManuallyDrop::new(spare);
        let len = spare.len();
        let ptr = spare.as_ptr();
        let orig: &mut $crate::GenericVec<_> = $orig;
        $crate::validate_spare(ptr, orig);
        let len = len + orig.len();
        $orig.set_len_unchecked(len);
    }};
}

#[doc(hidden)]
pub fn validate_spare<T>(spare_ptr: *const T, orig: &[T]) {
    debug_assert!(
        unsafe { orig.as_ptr().add(orig.len()) == spare_ptr },
        "Tried to use `save_spare!` with a `SliceVec` that was not obtained from `GenricVec::spare_capacity_mut`. \
         This is undefined behavior on release mode!"
    );
}

/// A vector type that can be backed up by a variety of different backends
/// including slices, arrays, and the heap.
#[repr(C)]
pub struct GenericVec<S: ?Sized + Storage> {
    len: usize,
    storage: S,
}

unsafe fn slice_assume_init_ref<T>(slice: &[MaybeUninit<T>]) -> &[T] {
    // SAFETY: casting `slice` to a `*const [T]` is safe since the caller guarantees that
    // `slice` is initialized, and `MaybeUninit` is guaranteed to have the same layout as `T`.
    // The pointer obtained is valid since it refers to memory owned by `slice` which is a
    // reference and thus guaranteed to be valid for reads.
    unsafe { &*(slice as *const [MaybeUninit<T>] as *const [T]) }
}

unsafe fn slice_assume_init_mut<T>(slice: &mut [MaybeUninit<T>]) -> &mut [T] {
    // SAFETY: similar to safety notes for `slice_get_ref`, but we have a
    // mutable reference which is also guaranteed to be valid for writes.
    unsafe { &mut *(slice as *mut [MaybeUninit<T>] as *mut [T]) }
}

impl<S: ?Sized + Storage> Deref for GenericVec<S> {
    type Target = [S::Item];

    fn deref(&self) -> &Self::Target {
        let len = self.len;
        // The first `len` elements are guaranteed to be initialized
        // as part of the guarantee on `self.set_len_unchecked`
        unsafe { slice_assume_init_ref(&self.storage.as_ref()[..len]) }
    }
}

impl<S: ?Sized + Storage> DerefMut for GenericVec<S> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        let len = self.len;
        // The first `len` elements are guaranteed to be initialized
        // as part of the guarantee on `self.set_len_unchecked`
        unsafe { slice_assume_init_mut(&mut self.storage.as_mut()[..len]) }
    }
}

impl<S: ?Sized + Storage> Drop for GenericVec<S> {
    fn drop(&mut self) {
        // The first `len` elements are guaranteed to be initialized
        // as part of the guarantee on `self.set_len_unchecked`
        // These elements should be dropped when the `GenericVec` gets dropped/
        // The storage will clean it's self up on drop
        unsafe { ptr::drop_in_place(self.as_mut_slice()) }
    }
}

impl<S: Storage> GenericVec<S> {
    /// Create a new empty `GenericVec` with the given backend
    ///
    /// ```rust
    /// use cl_generic_vec::{ArrayVec, uninit_array};
    /// let vec = ArrayVec::<i32, 4>::with_storage(uninit_array());
    /// ```
    pub fn with_storage(storage: S) -> Self { Self::with_storage_len(storage, 0) }

    fn with_storage_len(storage: S, len: usize) -> Self { Self { storage, len } }
}

impl<S: raw::StorageWithCapacity> GenericVec<S> {
    /// Create a new empty `GenericVec` with the backend with at least the given capacity
    pub fn with_capacity(capacity: usize) -> Self { Self::with_storage(S::with_capacity(capacity)) }

    #[inline]
    #[allow(non_snake_case)]
    fn __with_capacity__const_capacity_checked(capacity: usize, old_capacity: Option<usize>) -> Self {
        Self::with_storage(S::__with_capacity__const_capacity_checked(capacity, old_capacity))
    }
}

unsafe fn tm_array<T, U, const N: usize>(array: [T; N]) -> [U; N] {
    let array = ManuallyDrop::new(array);
    unsafe { array.as_ptr().cast::<[U; N]>().read() }
}

impl<T, const N: usize> ArrayVec<T, N> {
    /// Create a new empty `ArrayVec`
    pub fn new() -> Self {
        let uninit = MaybeUninit::<[T; N]>::uninit();
        let uninit = unsafe { tm_array::<T, MaybeUninit<T>, N>(MaybeUninit::assume_init(uninit)) };
        Self::with_storage(uninit)
    }

    /// Create a new full `ArrayVec`
    pub fn from_array(array: [T; N]) -> Self {
        // Safety:
        // The two arrays have exactly the same representation
        // and the code is taking ownership of the maybeuninit structure,
        // specifying an initialised count of N, so it's still known to be
        // initialised.
        let storage = unsafe { tm_array(array) };
        Self { len: N, storage }
    }

    /// Convert this `ArrayVec` into an array
    ///
    /// # Panics
    ///
    /// Panics if the the collection is not full
    pub fn into_array(self) -> [T; N] {
        match self.try_into_array() {
            Ok(a) => a,
            _ => panic!("ArrayVec is not full"),
        }
    }

    /// Convert this `ArrayVec` into an array
    ///
    /// # Errors
    ///
    /// errors if the the collection is not full
    pub fn try_into_array(self) -> Result<[T; N], Self> {
        if self.is_full() {
            let this = ManuallyDrop::new(self);

            // Safety: we have just asserted that the full array is initialised
            // unsafe { MaybeUninit::array_assume_init(self.storage) }
            Ok(unsafe { this.storage.as_ptr().cast::<[T; N]>().read() })
        } else {
            Err(self)
        }
    }
}

#[cfg(feature = "alloc")]
#[cfg_attr(doc, doc(cfg(feature = "alloc")))]
impl<T> HeapVec<T> {
    /// Create a new empty `HeapVec`
    pub fn new() -> Self {
        Self {
            len: 0,
            storage: Box::<[MaybeUninit<T>]>::default(),
        }
    }
}

#[cfg(any(doc, all(feature = "nightly", feature = "alloc")))]
#[cfg_attr(doc, doc(cfg(all(feature = "nightly", feature = "alloc"))))]
impl<T, A: std::alloc::Allocator> HeapVec<T, A> {
    /// Create a new empty `HeapVec` with the given allocator
    pub fn with_alloc(alloc: A) -> Self { Self::with_storage(Box::new_uninit_slice_in(0, alloc)) }
}

impl<'a, T> SliceVec<'a, T> {
    /// Create a new empty `SliceVec`
    pub fn new(slice: &'a mut [MaybeUninit<T>]) -> Self { Self::with_storage(slice) }

    /// Create a new full `SliceVec`
    pub fn full(slice: &'a mut [T]) -> Self {
        let len = slice.len();
        let storage = unsafe { &mut *(slice as *mut [T] as *mut [MaybeUninit<T>]) };
        Self::with_storage_len(storage, len)
    }
}

impl<S: Storage> GenericVec<S> {
    /// Convert a `GenericVec` into a length-storage pair
    pub fn into_raw_parts(self) -> (usize, S) {
        let this = core::mem::ManuallyDrop::new(self);
        unsafe { (this.len, core::ptr::read(&this.storage)) }
    }

    /// Create a `GenericVec` from a length-storage pair
    ///
    /// # Safety
    ///
    /// the length must be less than `raw.capacity()` and
    /// all elements in the range `0..length`, must be initialized
    ///
    /// # Panic
    ///
    /// If the given storage cannot hold type `T`, then this method will panic
    #[cfg(not(feature = "nightly"))]
    pub unsafe fn from_raw_parts(len: usize, storage: S) -> Self { Self { storage, len } }
}

#[cfg(feature = "nightly")]
impl<S: Storage> GenericVec<S> {
    /// Create a `GenericVec` from a length-storage pair
    ///
    /// Note: this is only const with the `nightly` feature enabled
    ///
    /// # Safety
    ///
    /// the length must be less than `raw.capacity()` and
    /// all elements in the range `0..length`, must be initialized
    ///
    /// # Panic
    ///
    /// If the given storage cannot hold type `T`, then this method will panic
    pub const unsafe fn from_raw_parts(len: usize, storage: S) -> Self { Self { len, storage } }
}

impl<S: ?Sized + Storage> GenericVec<S> {
    /// Returns the number of elements the vector can hold without reallocating or panicing.
    pub fn capacity(&self) -> usize {
        if core::mem::size_of::<S::Item>() == 0 {
            isize::MAX as usize
        } else {
            self.storage.as_ref().len()
        }
    }

    /// Returns true if and only if the vector contains no elements.
    pub fn is_empty(&self) -> bool { self.len() == 0 }

    /// Returns true if and only if the vector's length is equal to it's capacity.
    pub fn is_full(&self) -> bool { self.len() == self.capacity() }

    /// Returns the length of the spare capacity of the `GenericVec`
    pub fn remaining_capacity(&self) -> usize { self.capacity().wrapping_sub(self.len()) }

    /// Set the length of a vector
    ///
    /// # Safety
    ///
    /// * `new_len` must be less than or equal to `capacity()`.
    /// * The elements at `old_len..new_len` must be initialized.
    pub unsafe fn set_len_unchecked(&mut self, len: usize) { self.len = len; }

    /// Set the length of a vector
    ///
    /// # Panics
    /// If the length is set to be larger than the capacity
    pub fn set_len(&mut self, len: usize) {
        // Safety
        //
        // The storage only contains initialized data, and we check that
        // the given length is smaller than the capacity
        unsafe {
            assert!(
                len <= self.capacity(),
                "Tried to set the length to larger than the capacity"
            );
            self.set_len_unchecked(len);
        }
    }

    /// Extracts a slice containing the entire vector.
    ///
    /// Equivalent to &s[..].
    pub fn as_slice(&self) -> &[S::Item] { self }

    /// Extracts a mutable slice containing the entire vector.
    ///
    /// Equivalent to &mut s[..].
    pub fn as_mut_slice(&mut self) -> &mut [S::Item] { self }

    /// Returns the underlying storage
    pub fn storage(&self) -> &S { &self.storage }

    /// Returns the underlying storage
    ///
    /// # Safety
    ///
    /// You must not replace the storage
    pub unsafe fn storage_mut(&mut self) -> &mut S { &mut self.storage }

    /// Returns the remaining spare capacity of the vector as
    /// a [`SliceVec<'_, T>`](SliceVec).
    ///
    /// Keep in mind that the [`SliceVec<'_, T>`](SliceVec) will drop all elements
    /// that you push into it when it goes out of scope! If you want
    /// these modifications to persist then you should use [`save_spare`]
    /// to persist these writes.
    ///
    /// ```
    /// let mut vec = cl_generic_vec::ArrayVec::<i32, 16>::new();
    ///
    /// let mut spare = vec.spare_capacity_mut();
    /// spare.push(0);
    /// spare.push(2);
    /// drop(spare);
    /// assert_eq!(vec, []);
    ///
    /// let mut spare = vec.spare_capacity_mut();
    /// spare.push(0);
    /// spare.push(2);
    /// unsafe { cl_generic_vec::save_spare!(spare, &mut vec) }
    /// assert_eq!(vec, [0, 2]);
    /// ```
    pub fn spare_capacity_mut(&mut self) -> SliceVec<'_, S::Item> {
        // Safety
        //
        // The elements from `len..capacity` are guaranteed to be contain
        // `A::BufferItem`s, as per `Storage`'s safety requirements
        unsafe {
            let len = self.len();
            let cap = self.capacity();
            SliceVec::new(core::slice::from_raw_parts_mut(
                self.as_mut().as_mut_ptr().add(len).cast(),
                cap.wrapping_sub(len),
            ))
        }
    }

    /// Reserve enough space for at least `additional` elements
    ///
    /// # Panics
    ///
    /// May panic or abort if it isn't possible to allocate enough space for
    /// `additional` more elements
    #[inline]
    pub fn reserve(&mut self, additional: usize) {
        #[cold]
        #[inline(never)]
        fn allocation_failure(additional: usize) -> ! {
            panic!("Tried to allocate: {} more space and failed", additional)
        }

        if self.remaining_capacity() < additional {
            self.storage.reserve(match self.len().checked_add(additional) {
                Some(new_capacity) => new_capacity,
                None => allocation_failure(additional),
            });
        }
    }

    /// Try to reserve enough space for at least `additional` elements, and returns `Err(_)`
    /// if it's not possible to reserve enough space
    #[inline]
    pub fn try_reserve(&mut self, additional: usize) -> AllocResult {
        if self.remaining_capacity() < additional {
            match self.len().checked_add(additional) {
                Some(new_capacity) => self.storage.try_reserve(new_capacity),
                None => Err(AllocError),
            }
        } else {
            Ok(())
        }
    }

    /// Shortens the vector, keeping the first len elements and dropping the rest.
    ///
    /// If len is greater than the vector's current length, this has no effect.
    ///
    /// Note that this method has no effect on the allocated capacity of the vector.
    pub fn truncate(&mut self, len: usize) {
        if let Some(diff) = self.len().checked_sub(len) {
            // # Safety
            //
            // * the given length is smaller than the current length, so
            //   all the elements must be initialized
            // * the elements from `len..self.len()` are valid,
            //   and should be dropped
            unsafe {
                self.set_len_unchecked(len);
                let ptr = self.as_mut_ptr().add(len);
                let len = diff;
                core::ptr::drop_in_place(core::slice::from_raw_parts_mut(ptr, len));
            }
        }
    }

    /// Grows the `GenericVec` in-place by additional elements.
    ///
    /// This method requires `T` to implement `Clone`, in order to be able to clone
    /// the passed value. If you need more flexibility (or want to rely on Default instead of `Clone`),
    /// use [`GenericVec::grow_with`].
    ///
    /// # Panic
    ///
    /// May panic or reallocate if the collection is full
    ///
    /// # Panic behavor
    ///
    /// If `T::clone` panics, then all added items will be dropped. This is different
    /// from `std`, where on panic, items will stay in the `Vec`. This behavior
    /// is unstable, and may change in the future.
    pub fn grow(&mut self, additional: usize, value: S::Item)
    where
        S::Item: Clone,
    {
        self.reserve(additional);
        // # Safety
        //
        // * we reserved enough space
        unsafe { extension::Extension::grow(self, additional, value) }
    }

    /// Grows the `GenericVec` in-place by additional elements.
    ///
    /// This method uses a closure to create new values on every push.
    /// If you'd rather `Clone` a given value, use `GenericVec::resize`.
    /// If you want to use the `Default` trait to generate values, you
    /// can pass `Default::default` as the second argument.
    ///
    /// # Panic
    ///
    /// May panic or reallocate if the collection is full
    ///
    /// # Panic behavor
    ///
    /// If `F` panics, then all added items will be dropped. This is different
    /// from `std`, where on panic, items will stay in the `Vec`. This behavior
    /// is unstable, and may change in the future.
    pub fn grow_with<F>(&mut self, additional: usize, mut value: F)
    where
        F: FnMut() -> S::Item,
    {
        // Safety
        //
        // * we reserve enough space for `additional` elements
        // * we use `spare_capacity_mut` to ensure that the items are dropped,
        //   even on panic
        // * the `ptr` always stays in bounds

        self.reserve(additional);
        let mut writer = self.spare_capacity_mut();

        for _ in 0..additional {
            unsafe {
                writer.push_unchecked(value());
            }
        }

        unsafe {
            save_spare!(writer, self);
        }
    }

    /// Resizes the [`GenericVec`] in-place so that `len` is equal to `new_len`.
    ///
    /// If `new_len` is greater than `len`, the [`GenericVec`] is extended by the difference,
    /// with each additional slot filled with value. If `new_len` is less than `len`,
    /// the [`GenericVec`] is simply truncated.
    ///
    /// If you know that `new_len` is larger than `len`, then use [`GenericVec::grow`]
    ///
    /// If you know that `new_len` is less than `len`, then use [`GenericVec::truncate`]
    ///
    /// This method requires `T` to implement `Clone`, in order to be able to clone
    /// the passed value. If you need more flexibility (or want to rely on Default
    /// instead of `Clone`), use [`GenericVec::resize_with`].
    ///
    /// # Panic
    ///
    /// May panic or reallocate if the collection is full
    ///
    /// # Panic behavor
    ///
    /// If `F` panics, then all added items will be dropped. This is different
    /// from `std`, where on panic, items will stay in the `Vec`. This behavior
    /// is unstable, and may change in the future.
    pub fn resize(&mut self, new_len: usize, value: S::Item)
    where
        S::Item: Clone,
    {
        match new_len.checked_sub(self.len()) {
            Some(0) => (),
            Some(additional) => self.grow(additional, value),
            None => self.truncate(new_len),
        }
    }

    /// Resizes the [`GenericVec`] in-place so that len is equal to `new_len`.
    ///
    /// If `new_len` is greater than `len`, the [`GenericVec`] is extended by the
    /// difference, with each additional slot filled with the result of calling
    /// the closure `f`. The return values from `f` will end up in the [`GenericVec`]
    /// in the order they have been generated.
    ///
    /// If `new_len` is less than `len`, the [`GenericVec`] is simply truncated.
    ///
    /// If you know that `new_len` is larger than `len`, then use [`GenericVec::grow_with`]
    ///
    /// If you know that `new_len` is less than `len`, then use [`GenericVec::truncate`]
    ///
    /// This method uses a closure to create new values on every push. If you'd
    /// rather [`Clone`] a given value, use [`GenericVec::resize`]. If you want to
    /// use the [`Default`] trait to generate values, you can pass [`Default::default`]
    /// as the second argument.
    ///
    /// # Panic
    ///
    /// May panic or reallocate if the collection is full
    ///
    /// # Panic behavor
    ///
    /// If `F` panics, then all added items will be dropped. This is different
    /// from `std`, where on panic, items will stay in the `Vec`. This behavior
    /// is unstable, and may change in the future.
    pub fn resize_with<F: FnMut() -> S::Item>(&mut self, new_len: usize, value: F) {
        match new_len.checked_sub(self.len()) {
            Some(0) => (),
            Some(additional) => self.grow_with(additional, value),
            None => self.truncate(new_len),
        }
    }

    /// Clears the vector, removing all values.
    ///
    /// Note that this method has no effect on the allocated capacity of the vector.
    pub fn clear(&mut self) { self.truncate(0); }

    /// Appends an element to the back of a collection.
    ///
    /// # Panic
    ///
    /// May panic or reallocate if the collection is full
    pub fn push(&mut self, value: S::Item) -> &mut S::Item {
        if self.len() == self.capacity() {
            self.reserve(1);
        }

        // Safety
        //
        // * we reserve enough space for 1 more element
        unsafe { self.push_unchecked(value) }
    }

    /// Appends the array to the back of a collection.
    ///
    /// # Panic
    ///
    /// May panic or reallocate if the collection has less than N elements remaining
    #[cfg(any(doc, feature = "nightly"))]
    pub fn push_array<const N: usize>(&mut self, value: [S::Item; N]) -> &mut [S::Item; N] {
        self.reserve(N);

        // Safety
        //
        // * we reserve enough space for N more elements
        unsafe { self.push_array_unchecked(value) }
    }

    /// Inserts an element at position index within the vector,
    /// shifting all elements after it to the right.
    ///
    /// # Panics
    ///
    /// * May panic or reallocate if the collection is full
    /// * Panics if index > len.
    pub fn insert(&mut self, index: usize, value: S::Item) -> &mut S::Item {
        #[cold]
        #[inline(never)]
        fn insert_fail(index: usize, len: usize) -> ! {
            panic!("Tried to insert at {}, but length is {}", index, len);
        }

        if index > self.len() {
            insert_fail(index, self.len())
        }

        if self.is_full() {
            self.reserve(1);
        }

        // Safety
        //
        // * we reserve enough space for 1 more element
        // * we verify that index is in bounds
        unsafe { self.insert_unchecked(index, value) }
    }

    /// Inserts the array at position index within the vector,
    /// shifting all elements after it to the right.
    ///
    /// # Panics
    ///
    /// * May panic or reallocate if the collection has less than N elements remaining
    /// * Panics if index > len.
    #[cfg(any(doc, feature = "nightly"))]
    pub fn insert_array<const N: usize>(&mut self, index: usize, value: [S::Item; N]) -> &mut [S::Item; N] {
        #[cold]
        #[inline(never)]
        fn insert_array_fail(index: usize, size: usize, len: usize) -> ! {
            panic!(
                "Tried to insert array of length {} at {}, but length is {}",
                size, index, len
            );
        }

        if index > self.len() {
            insert_array_fail(index, N, self.len())
        }

        self.reserve(N);

        // Safety
        //
        // * we reserve enough space for N more elements
        // * we verify that index is in bounds
        unsafe { self.insert_array_unchecked(index, value) }
    }

    /// Removes the last element from a vector and returns it
    ///
    /// # Panics
    ///
    /// Panics if the collection is empty
    pub fn pop(&mut self) -> S::Item {
        #[cold]
        #[inline(never)]
        fn pop_fail() -> ! {
            panic!("Tried to pop an element from an empty vector",);
        }

        if self.is_empty() {
            pop_fail()
        }

        // Safety
        //
        // * we verify we are not empty
        unsafe { self.pop_unchecked() }
    }

    /// Removes the last `N` elements from a vector and returns it
    ///
    /// # Panics
    ///
    /// Panics if the collection contains less than `N` elements in it
    #[cfg(any(doc, feature = "nightly"))]
    pub fn pop_array<const N: usize>(&mut self) -> [S::Item; N] {
        #[cold]
        #[inline(never)]
        fn pop_array_fail(size: usize, len: usize) -> ! {
            panic!("Tried to pop an array of size {}, a vector of length {}", size, len);
        }

        if self.len() < N {
            pop_array_fail(N, self.len())
        }

        // Safety
        //
        // * we verify we have at least N elements
        unsafe { self.pop_array_unchecked() }
    }

    /// Removes and returns the element at position index within the vector,
    /// shifting all elements after it to the left.
    ///
    /// # Panics
    ///
    /// Panics if `index` is out of bounds.
    pub fn remove(&mut self, index: usize) -> S::Item {
        #[cold]
        #[inline(never)]
        fn remove_fail(index: usize, len: usize) -> ! {
            panic!("Tried to remove an element at {}, but length is {}", index, len);
        }

        if index > self.len() {
            remove_fail(index, self.len())
        }

        // Safety
        //
        // * we verify that the index is in bounds
        unsafe { self.remove_unchecked(index) }
    }

    /// Removes and returns `N` elements at position index within the vector,
    /// shifting all elements after it to the left.
    ///
    /// # Panics
    ///
    /// Panics if `index` is out of bounds or if `index + N > len()`
    #[cfg(any(doc, feature = "nightly"))]
    pub fn remove_array<const N: usize>(&mut self, index: usize) -> [S::Item; N] {
        #[cold]
        #[inline(never)]
        fn remove_array_fail(index: usize, size: usize, len: usize) -> ! {
            panic!(
                "Tried to remove an array length {} at {}, but length is {}",
                size, index, len
            );
        }

        if self.len() < index || self.len().wrapping_sub(index) < N {
            remove_array_fail(index, N, self.len())
        }

        // Safety
        //
        // * we verify that the index is in bounds
        // * we verify that there are at least `N` elements
        //   after the index
        unsafe { self.remove_array_unchecked(index) }
    }

    /// Removes an element from the vector and returns it.
    ///
    /// The removed element is replaced by the last element of the vector.
    ///
    /// This does not preserve ordering, but is O(1).
    ///
    /// # Panics
    ///
    /// Panics if `index` is out of bounds.
    pub fn swap_remove(&mut self, index: usize) -> S::Item {
        #[cold]
        #[inline(never)]
        fn swap_remove_fail(index: usize, len: usize) -> ! {
            panic!("Tried to remove an element at {}, but length is {}", index, len);
        }

        if index > self.len() {
            swap_remove_fail(index, self.len())
        }

        // Safety
        //
        // * we verify that the index is in bounds
        unsafe { self.swap_remove_unchecked(index) }
    }

    /// Tries to append an element to the back of a collection.
    /// Returns the `Err(value)` if the collection is full
    ///
    /// Guaranteed to not panic/abort/allocate
    pub fn try_push(&mut self, value: S::Item) -> Result<&mut S::Item, S::Item> {
        if self.is_full() {
            Err(value)
        } else {
            // Safety
            //
            // * we reserve enough space for 1 more element
            unsafe { Ok(self.push_unchecked(value)) }
        }
    }

    /// Tries to append an array to the back of a collection.
    /// Returns the `Err(value)` if the collection doesn't have enough remaining capacity
    /// to hold `N` elements.
    ///
    /// Guaranteed to not panic/abort/allocate
    #[cfg(any(doc, feature = "nightly"))]
    pub fn try_push_array<const N: usize>(&mut self, value: [S::Item; N]) -> Result<&mut [S::Item; N], [S::Item; N]> {
        if self.remaining_capacity() < N {
            Err(value)
        } else {
            // Safety
            //
            // * we reserve enough space for N more elements
            unsafe { Ok(self.push_array_unchecked(value)) }
        }
    }

    /// Inserts an element at position index within the vector,
    /// shifting all elements after it to the right.
    /// Returns the `Err(value)` if the collection is full or index is out of bounds
    ///
    /// Guaranteed to not panic/abort/allocate
    pub fn try_insert(&mut self, index: usize, value: S::Item) -> Result<&mut S::Item, S::Item> {
        if self.is_full() || index > self.len() {
            Err(value)
        } else {
            // Safety
            //
            // * we reserve enough space for 1 more element
            // * we verify that index is in bounds
            unsafe { Ok(self.insert_unchecked(index, value)) }
        }
    }

    /// Inserts an array at position index within the vector,
    /// shifting all elements after it to the right.
    /// Returns the `Err(value)` if the collection doesn't have enough remaining capacity
    /// to hold `N` elements or index is out of bounds
    ///
    /// Guaranteed to not panic/abort/allocate
    #[cfg(any(doc, feature = "nightly"))]
    pub fn try_insert_array<const N: usize>(
        &mut self,
        index: usize,
        value: [S::Item; N],
    ) -> Result<&mut [S::Item; N], [S::Item; N]> {
        if self.capacity().wrapping_sub(self.len()) < N || index > self.len() {
            Err(value)
        } else {
            // Safety
            //
            // * we reserve enough space for N more elements
            // * we verify that index is in bounds
            unsafe { Ok(self.insert_array_unchecked(index, value)) }
        }
    }

    /// Removes the last element from a vector and returns it,
    /// Returns `None` if the collection is empty
    ///
    /// Guaranteed to not panic/abort/allocate
    pub fn try_pop(&mut self) -> Option<S::Item> {
        if self.is_empty() {
            None
        } else {
            // Safety
            //
            // * we verify we are not empty
            unsafe { Some(self.pop_unchecked()) }
        }
    }

    /// Removes the last `N` elements from a vector and returns it,
    /// Returns `None` if the collection is has less than N elements
    ///
    /// Guaranteed to not panic/abort/allocate
    #[cfg(any(doc, feature = "nightly"))]
    pub fn try_pop_array<const N: usize>(&mut self) -> Option<[S::Item; N]> {
        if self.is_empty() {
            None
        } else {
            // Safety
            //
            // * we verify we have at least N elements
            unsafe { Some(self.pop_array_unchecked()) }
        }
    }

    /// Removes and returns the element at position index within the vector,
    /// shifting all elements after it to the left.
    /// Returns `None` if collection is empty or `index` is out of bounds.
    ///
    /// Guaranteed to not panic/abort/allocate
    pub fn try_remove(&mut self, index: usize) -> Option<S::Item> {
        if self.len() < index {
            None
        } else {
            // Safety
            //
            // * we verify that the index is in bounds
            unsafe { Some(self.remove_unchecked(index)) }
        }
    }

    /// Removes and returns the element at position index within the vector,
    /// shifting all elements after it to the left.
    /// Returns `None` if the collection is has less than N elements
    /// or `index` is out of bounds.
    ///
    /// Guaranteed to not panic/abort/allocate
    #[cfg(any(doc, feature = "nightly"))]
    pub fn try_remove_array<const N: usize>(&mut self, index: usize) -> Option<[S::Item; N]> {
        if self.len() < index || self.len().wrapping_sub(index) < N {
            None
        } else {
            // Safety
            //
            // * we verify that the index is in bounds
            // * we verify that there are at least `N` elements
            //   after the index
            unsafe { Some(self.remove_array_unchecked(index)) }
        }
    }

    /// Removes an element from the vector and returns it.
    /// Returns `None` if collection is empty or `index` is out of bounds.
    ///
    /// The removed element is replaced by the last element of the vector.
    ///
    /// This does not preserve ordering, but is O(1).
    ///
    /// Guaranteed to not panic/abort/allocate
    pub fn try_swap_remove(&mut self, index: usize) -> Option<S::Item> {
        if index < self.len() {
            // Safety
            //
            // * we verify that the index is in bounds
            unsafe { Some(self.swap_remove_unchecked(index)) }
        } else {
            None
        }
    }

    /// Appends an element to the back of a collection.
    ///
    /// # Safety
    ///
    /// the collection must not be full
    pub unsafe fn push_unchecked(&mut self, value: S::Item) -> &mut S::Item {
        debug_assert_ne!(
            self.len(),
            self.capacity(),
            "Tried to `push_unchecked` past capacity! This is UB in release mode"
        );

        // Safety
        //
        // the collection isn't full, so `ptr.add(len)` is valid to write
        unsafe {
            let len = self.len();
            self.set_len_unchecked(len.wrapping_add(1));
            let ptr = self.as_mut_ptr().add(len);
            ptr.write(value);
            &mut *ptr
        }
    }

    /// Appends the array to the back of a collection.
    ///
    /// # Safety
    ///
    /// the collection's remaining capacity must be at least N
    #[cfg(any(doc, feature = "nightly"))]
    pub unsafe fn push_array_unchecked<const N: usize>(&mut self, value: [S::Item; N]) -> &mut [S::Item; N] {
        match S::CONST_CAPACITY {
            Some(n) if n < N => {
                panic!("Tried to push an array larger than the maximum capacity of the vector!")
            }
            _ => (),
        }

        // Safety
        //
        // the collection has at least N remaining elements of capacity left,
        // so `ptr.add(len)` is valid to write `N` elements
        unsafe {
            let len = self.len();
            self.set_len_unchecked(len.wrapping_add(N));
            let ptr = self.as_mut_ptr();
            let out = ptr.add(len) as *mut [S::Item; N];
            out.write(value);
            &mut *out
        }
    }

    /// Inserts an element at position index within the vector,
    /// shifting all elements after it to the right.
    ///
    /// # Safety
    ///
    /// * the collection is must not be full
    /// * the index must be in bounds
    pub unsafe fn insert_unchecked(&mut self, index: usize, value: S::Item) -> &mut S::Item {
        unsafe {
            debug_assert_ne!(
                self.len(),
                self.capacity(),
                "Tried to `insert_unchecked` past capacity! This is UB in release mode"
            );

            // Safety
            //
            // * the index is in bounds
            // * the collection is't full so `ptr.add(len)` is valid to write 1 element
            let len = self.len();
            self.set_len_unchecked(len.wrapping_add(1));
            let ptr = self.as_mut().as_mut_ptr().add(index);
            ptr.add(1).copy_from(ptr, len.wrapping_sub(index));
            ptr.write(value);
            &mut *ptr
        }
    }

    /// Inserts an array at position index within the vector,
    /// shifting all elements after it to the right.
    ///
    /// # Safety
    ///
    /// * the collection's remaining capacity must be at least N
    /// * hte index must be in bounds
    #[cfg(any(doc, feature = "nightly"))]
    pub unsafe fn insert_array_unchecked<const N: usize>(
        &mut self,
        index: usize,
        value: [S::Item; N],
    ) -> &mut [S::Item; N] {
        match S::CONST_CAPACITY {
            Some(n) if n < N => {
                panic!("Tried to push an array larger than the maximum capacity of the vector!")
            }
            _ => (),
        }

        // Safety
        //
        // * the index is in bounds
        // * the collection has at least N remaining elements of capacity left,
        //   so `ptr.add(len)` is valid to write `N` elements
        unsafe {
            let len = self.len();
            self.set_len_unchecked(len.wrapping_add(N));
            let ptr = self.as_mut_ptr();
            let dist = len.wrapping_sub(index);

            let out = ptr.add(index);
            out.add(N).copy_from(out, dist);
            let out = out as *mut [S::Item; N];
            out.write(value);
            &mut *out
        }
    }

    /// Removes the last element from a vector and returns it
    ///
    /// # Safety
    ///
    /// the collection must not be empty
    pub unsafe fn pop_unchecked(&mut self) -> S::Item {
        let len = self.len();
        debug_assert_ne!(
            len, 0,
            "Tried to `pop_unchecked` an empty array vec! This is UB in release mode"
        );

        // Safety
        //
        // * the collection isn't empty, so `ptr.add(len - 1)` is valid to read
        unsafe {
            let len = len.wrapping_sub(1);
            self.set_len_unchecked(len);
            self.as_mut_ptr().add(len).read()
        }
    }

    /// Removes the last `N` elements from a vector and returns it
    ///
    /// # Safety
    ///
    /// The collection must contain at least `N` elements in it
    #[cfg(any(doc, feature = "nightly"))]
    pub unsafe fn pop_array_unchecked<const N: usize>(&mut self) -> [S::Item; N] {
        match S::CONST_CAPACITY {
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
        // Safety
        //
        // * the collection has at least `N` elements, so `ptr.add(len - N)` is valid to read `N` elements
        unsafe {
            let len = len.wrapping_sub(N);
            self.set_len_unchecked(len);
            self.as_mut_ptr().add(len).cast::<[S::Item; N]>().read()
        }
    }

    /// Removes and returns the element at position index within the vector,
    /// shifting all elements after it to the left.
    ///
    /// # Safety
    ///
    /// the collection must not be empty, and
    /// index must be in bounds
    pub unsafe fn remove_unchecked(&mut self, index: usize) -> S::Item {
        let len = self.len();

        debug_assert!(
            index <= len,
            "Tried to remove an element at index {} from a {} length vector! This is UB in release mode",
            index,
            len,
        );

        // Safety
        //
        // * the index is in bounds
        // * the collection isn't empty, so `ptr.add(len - index - 1)` is valid to read
        unsafe {
            self.set_len_unchecked(len.wrapping_sub(1));
            let ptr = self.as_mut().as_mut_ptr().add(index);
            let value = ptr.read();
            ptr.copy_from(ptr.add(1), len.wrapping_sub(index).wrapping_sub(1));
            value
        }
    }

    /// Removes and returns the element at position index within the vector,
    /// shifting all elements after it to the left.
    ///
    /// # Safety
    ///
    /// the collection must contain at least N elements, and
    /// index must be in bounds
    #[cfg(any(doc, feature = "nightly"))]
    pub unsafe fn remove_array_unchecked<const N: usize>(&mut self, index: usize) -> [S::Item; N] {
        match S::CONST_CAPACITY {
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

        // Safety
        //
        // * the index is in bounds
        // * the collection isn't empty, so `ptr.add(len - index - N)` is valid to read `N` elements
        unsafe {
            self.set_len_unchecked(len.wrapping_sub(N));
            let ptr = self.as_mut_ptr().add(index);
            let value = ptr.cast::<[S::Item; N]>().read();
            if N != 0 {
                ptr.copy_from(ptr.add(N), len.wrapping_sub(index).wrapping_sub(N));
            }
            value
        }
    }

    /// Removes an element from the vector and returns it.
    ///
    /// The removed element is replaced by the last element of the vector.
    ///
    /// This does not preserve ordering, but is O(1).
    ///
    /// # Safety
    ///
    /// the `index` must be in bounds
    pub unsafe fn swap_remove_unchecked(&mut self, index: usize) -> S::Item {
        // Safety
        //
        // * the index is in bounds
        // * the collection isn't empty
        unsafe {
            let len = self.len();
            self.set_len_unchecked(len.wrapping_sub(1));
            let ptr = self.as_mut().as_mut_ptr();
            let at = ptr.add(index);
            let end = ptr.add(len.wrapping_sub(1));
            let value = at.read();
            at.copy_from(end, 1);
            value
        }
    }

    /// Splits the collection into two at the given index.
    ///
    /// Returns a newly allocated vector containing the elements in the range `[at, len)`.
    /// After the call, the original vector will be left containing the elements `[0, at)`
    /// with its previous capacity unchanged.
    ///
    /// ```rust
    /// # use cl_generic_vec::{gvec, SliceVec, uninit_array};
    /// # let mut vec_buf = uninit_array::<_, 3>();
    /// # let mut vec2_buf = uninit_array::<_, 5>();
    /// # let mut vec: SliceVec<_> = SliceVec::new(&mut vec_buf); vec.extend([1, 2, 3].iter().copied());
    /// # let mut vec2: SliceVec<_> = SliceVec::new(&mut vec2_buf); vec2.extend([4, 5, 6].iter().copied());
    /// assert_eq!(vec, [1, 2, 3]);
    /// assert_eq!(vec2, [4, 5, 6]);
    /// vec.split_off_into(1, &mut vec2);
    /// assert_eq!(vec, [1]);
    /// assert_eq!(vec2, [4, 5, 6, 2, 3]);
    /// ```
    ///
    /// # Panics
    /// If the index is out of bounds
    pub fn split_off<B>(&mut self, index: usize) -> GenericVec<B>
    where
        B: raw::StorageWithCapacity<Item = S::Item>,
    {
        assert!(
            index <= self.len(),
            "Tried to split at index {}, but length is {}",
            index,
            self.len()
        );

        let mut vec =
            GenericVec::<B>::__with_capacity__const_capacity_checked(self.len().wrapping_sub(index), S::CONST_CAPACITY);

        self.split_off_into(index, &mut vec);

        vec
    }

    /// Splits the collection into two at the given index.
    ///
    /// Appends the elements from the range `[at, len)` to `other`.
    /// After the call, the original vector will be left containing the elements `[0, at)`
    /// with its previous capacity unchanged.
    ///
    /// ```rust
    /// # use cl_generic_vec::{gvec, SliceVec, uninit_array};
    /// # let mut vec_buf = uninit_array::<_, 3>();
    /// # let mut vec2_buf = uninit_array::<_, 5>();
    /// # let mut vec: SliceVec<_> = SliceVec::new(&mut vec_buf); vec.extend([1, 2, 3].iter().copied());
    /// # let mut vec2: SliceVec<_> = SliceVec::new(&mut vec2_buf); vec2.extend([4, 5, 6].iter().copied());
    /// assert_eq!(vec, [1, 2, 3]);
    /// assert_eq!(vec2, [4, 5, 6]);
    /// vec.split_off_into(1, &mut vec2);
    /// assert_eq!(vec, [1]);
    /// assert_eq!(vec2, [4, 5, 6, 2, 3]);
    /// ```
    ///
    /// # Panics
    /// If the index is out of bounds
    pub fn split_off_into<B>(&mut self, index: usize, other: &mut GenericVec<B>)
    where
        B: raw::Storage<Item = S::Item> + ?Sized,
    {
        assert!(
            index <= self.len(),
            "Tried to split at index {}, but length is {}",
            index,
            self.len()
        );

        unsafe {
            // Safety
            //
            // * the index is in bounds
            // * other has reserved enough space
            // * we ignore all elements after index
            let slice = self.get_unchecked(index..);
            other.reserve(slice.len());
            other.extend_from_slice_unchecked(slice);
            self.set_len_unchecked(index);
        }
    }

    /// Moves all the elements of `other` into `Self`, leaving `other` empty.
    ///
    /// Does not change the capacity of either collection.
    ///
    /// ```rust
    /// # use cl_generic_vec::{gvec, SliceVec, uninit_array};
    /// # let mut vec_buf = uninit_array::<_, 6>();
    /// # let mut vec2_buf = uninit_array::<_, 3>();
    /// # let mut vec: SliceVec<_> = SliceVec::new(&mut vec_buf); vec.extend([1, 2, 3].iter().copied());
    /// # let mut vec2: SliceVec<_> = SliceVec::new(&mut vec2_buf); vec2.extend([4, 5, 6].iter().copied());
    /// assert_eq!(vec, [1, 2, 3]);
    /// assert_eq!(vec2, [4, 5, 6]);
    /// vec.append(&mut vec2);
    /// assert_eq!(vec, [1, 2, 3, 4, 5, 6]);
    /// assert_eq!(vec2, []);
    /// ```
    ///
    /// # Panic
    ///
    /// May panic or reallocate if the collection is full
    pub fn append<B: Storage<Item = S::Item> + ?Sized>(&mut self, other: &mut GenericVec<B>) {
        other.split_off_into(0, self);
    }

    /// Convert the backing storage type, and moves all the elements in `self` to the new vector
    pub fn convert<B: raw::StorageWithCapacity<Item = S::Item>>(mut self) -> GenericVec<B>
    where
        S: Sized,
    {
        self.split_off(0)
    }

    /// Creates a raw cursor that can be used to remove elements in the specified range.
    /// Usage of [`RawCursor`](iter::RawCursor) is `unsafe` because it doesn't do any checks.
    /// [`RawCursor`](iter::RawCursor) is meant to be a low level tool to implement fancier
    /// iterators, like [`GenericVec::drain`], [`GenericVec::drain_filter`],
    /// or [`GenericVec::splice`].
    ///
    /// # Panic
    ///
    /// Panics if the starting point is greater than the end point or if the end point
    /// is greater than the length of the vector.
    #[inline]
    pub fn raw_cursor<R>(&mut self, range: R) -> iter::RawCursor<'_, S>
    where
        R: RangeBounds<usize>,
    {
        let range = slice::check_range(self.len(), range);
        iter::RawCursor::new(self, range)
    }

    /// Creates a cursor that can be used to remove elements in the specified range.
    ///
    /// # Panic
    ///
    /// Panics if the starting point is greater than the end point or if the end point
    /// is greater than the length of the vector.
    #[inline]
    pub fn cursor<R>(&mut self, range: R) -> iter::Cursor<'_, S>
    where
        R: RangeBounds<usize>,
    {
        iter::Cursor::new(self.raw_cursor(range))
    }

    /// Creates a draining iterator that removes the specified range in the
    /// vector and yields the removed items.
    ///
    /// When the iterator is dropped, all elements in the range are removed from
    /// the vector, even if the iterator was not fully consumed. If the iterator
    /// is not dropped (with `mem::forget` for example), it is unspecified how many
    /// elements are removed.
    ///
    /// # Panic
    ///
    /// Panics if the starting point is greater than the end point or if the end point
    /// is greater than the length of the vector.
    #[inline]
    pub fn drain<R>(&mut self, range: R) -> iter::Drain<'_, S>
    where
        R: RangeBounds<usize>,
    {
        iter::Drain::new(self.raw_cursor(range))
    }

    /// Creates an iterator which uses a closure to determine if an element should be removed.
    ///
    /// If the closure returns true, then the element is removed and yielded.
    /// If the closure returns false, the element will remain in the vector
    /// and will not be yielded by the iterator.
    ///
    /// # Panic
    ///
    /// Panics if the starting point is greater than the end point or if the end point
    /// is greater than the length of the vector.
    #[inline]
    pub fn drain_filter<R, F>(&mut self, range: R, f: F) -> iter::DrainFilter<'_, S, F>
    where
        R: RangeBounds<usize>,
        F: FnMut(&mut S::Item) -> bool,
    {
        iter::DrainFilter::new(self.raw_cursor(range), f)
    }

    /// Creates a splicing iterator that replaces the specified range in the vector with
    /// the given `replace_with` iterator and yields the removed items. `replace_with` does
    /// not need to be the same length as range.
    ///
    /// range is removed even if the iterator is not consumed until the end.
    ///
    /// It is unspecified how many elements are removed from the vector if the
    /// [`Splice`](iter::Splice) value is leaked.
    ///
    /// The input iterator `replace_with` is only consumed when the [`Splice`](iter::Splice)
    /// value is dropped
    ///
    /// # Panic
    ///
    /// Panics if the starting point is greater than the end point or if the end point
    /// is greater than the length of the vector.
    #[inline]
    pub fn splice<R, I>(&mut self, range: R, replace_with: I) -> iter::Splice<'_, S, I::IntoIter>
    where
        R: RangeBounds<usize>,
        I: IntoIterator<Item = S::Item>,
    {
        iter::Splice::new(self.raw_cursor(range), replace_with.into_iter())
    }

    /// Retains only the elements specified by the predicate.
    ///
    /// In other words, remove all elements `e` such that `f(e)` returns false.
    /// This method operates in place, visiting each element exactly once in
    /// the original order, and preserves the order of the retained elements.
    #[inline]
    pub fn retain<F>(&mut self, f: F)
    where
        F: FnMut(&mut S::Item) -> bool,
    {
        fn not<F: FnMut(&mut T) -> bool, T>(mut f: F) -> impl FnMut(&mut T) -> bool { move |value| !f(value) }
        self.drain_filter(.., not(f));
    }

    /// Shallow copies and appends all elements in a slice to the `GenericVec`.
    ///
    /// # Safety
    ///
    /// * You must not drop any of the elements in `slice`
    /// * There must be at least `slice.len()` remaining capacity in the vector
    pub unsafe fn extend_from_slice_unchecked(&mut self, slice: &[S::Item]) {
        debug_assert!(
            self.remaining_capacity() >= slice.len(),
            "Not enough capacity to hold the slice"
        );

        unsafe {
            let len = self.len();
            self.as_mut_ptr()
                .add(len)
                .copy_from_nonoverlapping(slice.as_ptr(), slice.len());
            self.set_len_unchecked(len.wrapping_add(slice.len()));
        }
    }

    /// Clones and appends all elements in a slice to the `GenericVec`.
    ///
    /// Iterates over the slice other, clones each element, and then appends
    /// it to this `GenericVec`. The other vector is traversed in-order.
    ///
    /// Note that this function is same as extend except that it is specialized
    /// to work with slices instead. If and when Rust gets specialization this
    /// function will likely be deprecated (but still available).
    ///
    /// # Panic behavor
    ///
    /// If `T::clone` panics, then all newly added items will be dropped. This is different
    /// from `std`, where on panic, newly added items will stay in the `Vec`. This behavior
    /// is unstable, and may change in the future.
    pub fn extend_from_slice(&mut self, slice: &[S::Item])
    where
        S::Item: Clone,
    {
        self.reserve(slice.len());

        // Safety
        //
        // We reserved enough space
        unsafe { extension::Extension::extend_from_slice(self, slice) }
    }

    /// Replaces all of the current elements with the ones in the slice
    ///
    /// equivalent to the following
    ///
    /// ```rust
    /// # let slice = [];
    /// # let mut buffer = cl_generic_vec::uninit_array::<_, 0>();
    /// # let mut vec = cl_generic_vec::SliceVec::<()>::new(&mut buffer);
    /// vec.clear();
    /// vec.extend_from_slice(&slice);
    /// ```
    ///
    /// # Panic
    ///
    /// May try to panic/reallocate if there is not enough capacity for the slice
    pub fn clone_from(&mut self, source: &[S::Item])
    where
        S::Item: Clone,
    {
        // If the `self` is longer than `source`, remove excess
        self.truncate(source.len());

        // `self` is now at most the same length as `source`
        //
        // * `init.len() == self.len()`
        // * tail is the rest of the `source`, in the case
        //     that `self` is smaller than `source`
        let (init, tail) = source.split_at(self.len());

        // Clone in the beginning, using `slice::clone_from_slice`
        self.clone_from_slice(init);

        // Append the remaining elements
        self.extend_from_slice(tail);
    }

    /// Removes all but the first of consecutive elements in the vector satisfying
    /// a given equality relation.
    ///
    /// The `same_bucket` function is passed references to two elements from the
    /// vector and must determine if the elements compare equal. The elements
    /// are passed in opposite order from their order in the slice, so if
    /// `same_bucket(a, b)` returns true, a is removed.
    ///
    /// If the vector is sorted, this removes all duplicates.
    pub fn dedup_by<F>(&mut self, same_bucket: F)
    where
        F: FnMut(&mut S::Item, &mut S::Item) -> bool,
    {
        let (a, _) = slice::partition_dedup_by(self.as_mut_slice(), same_bucket);
        let new_len = a.len();
        self.truncate(new_len);
    }

    /// Removes all but the first of consecutive elements in the vector that resolve to the same key.
    ///
    /// If the vector is sorted, this removes all duplicates.
    pub fn dedup_by_key<F, K>(&mut self, key: F)
    where
        F: FnMut(&mut S::Item) -> K,
        K: PartialEq,
    {
        #[inline]
        fn key_to_same_bucket<T, F, K>(mut f: F) -> impl FnMut(&mut T, &mut T) -> bool
        where
            F: FnMut(&mut T) -> K,
            K: PartialEq,
        {
            #[inline]
            move |a, b| {
                let a = f(a);
                let b = f(b);
                a == b
            }
        }

        self.dedup_by(key_to_same_bucket(key));
    }

    /// Removes all but the first of consecutive elements in the vector that resolve to the same key.
    ///
    /// If the vector is sorted, this removes all duplicates.
    pub fn dedup<F, K>(&mut self)
    where
        S::Item: PartialEq,
    {
        #[inline]
        fn eq_to_same_buckets<T, F>(mut f: F) -> impl FnMut(&mut T, &mut T) -> bool
        where
            F: FnMut(&T, &T) -> bool,
        {
            #[inline]
            move |a, b| f(a, b)
        }

        self.dedup_by(eq_to_same_buckets(PartialEq::eq));
    }
}
