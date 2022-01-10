//! The [`Iterator`] types that can be created from a [`GenericVec`]

mod cursor;
mod drain;
mod drain_filter;
mod into_iter;
mod raw_cursor;
mod splice;

pub use cursor::Cursor;
pub use drain::Drain;
pub use drain_filter::DrainFilter;
pub use into_iter::IntoIter;
pub use raw_cursor::RawCursor;
pub use splice::Splice;

use core::iter::FromIterator;

use crate::{
    raw::{Storage, StorageWithCapacity},
    GenericVec,
};

impl<V, S: StorageWithCapacity + Default> FromIterator<V> for GenericVec<S>
where
    Self: Extend<V>,
{
    #[inline]
    fn from_iter<I: IntoIterator<Item = V>>(iter: I) -> Self {
        let mut array = Self::default();
        array.extend(iter);
        array
    }
}

impl<S: ?Sized + Storage> Extend<S::Item> for GenericVec<S> {
    fn extend<I: IntoIterator<Item = S::Item>>(&mut self, iter: I) {
        let iter = iter.into_iter();
        let _ = self.try_reserve(iter.size_hint().0);
        #[allow(clippy::drop_ref)]
        iter.for_each(|item| drop(self.push(item)));
    }
}
