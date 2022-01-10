use crate::{iter::RawCursor, Storage};

use core::iter::FusedIterator;

/// This struct is created by [`GenericVec::drain`](crate::GenericVec::drain).
/// See its documentation for more.
pub struct Drain<'a, S: ?Sized + Storage> {
    raw: RawCursor<'a, S>,
}

impl<'a, S: ?Sized + Storage> Drain<'a, S> {
    pub(crate) fn new(raw: RawCursor<'a, S>) -> Self { Self { raw } }
}

impl<S: ?Sized + Storage> FusedIterator for Drain<'_, S> {}

impl<S: ?Sized + Storage> ExactSizeIterator for Drain<'_, S> {
    #[cfg(feature = "nightly")]
    fn is_empty(&self) -> bool { self.raw.is_empty() }
}

impl<S: ?Sized + Storage> Drop for Drain<'_, S> {
    fn drop(&mut self) { self.for_each(drop); }
}

impl<S: ?Sized + Storage> Iterator for Drain<'_, S> {
    type Item = S::Item;

    fn next(&mut self) -> Option<Self::Item> {
        if self.raw.is_empty() {
            None
        } else {
            unsafe { Some(self.raw.take_front()) }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.raw.len();
        (len, Some(len))
    }
}

impl<S: ?Sized + Storage> DoubleEndedIterator for Drain<'_, S> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.raw.is_empty() {
            None
        } else {
            unsafe { Some(self.raw.take_back()) }
        }
    }
}
