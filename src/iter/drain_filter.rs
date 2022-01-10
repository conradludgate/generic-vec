use crate::{iter::RawCursor, Storage};

use core::iter::FusedIterator;

/// This struct is created by [`GenericVec::drain_filter`](crate::GenericVec::drain_filter).
/// See its documentation for more.
pub struct DrainFilter<'a, S, F>
where
    S: ?Sized + Storage,
    F: FnMut(&mut S::Item) -> bool,
{
    raw: RawCursor<'a, S>,
    filter: F,
    panicking: bool,
}

struct SetOnDrop<'a>(&'a mut bool);

impl<'a> Drop for SetOnDrop<'a> {
    fn drop(&mut self) { *self.0 = true; }
}

impl<'a, S, F> DrainFilter<'a, S, F>
where
    S: ?Sized + Storage,
    F: FnMut(&mut S::Item) -> bool,
{
    pub(crate) fn new(raw: RawCursor<'a, S>, filter: F) -> Self {
        Self {
            raw,
            filter,
            panicking: false,
        }
    }
}

impl<S, F> Drop for DrainFilter<'_, S, F>
where
    S: ?Sized + Storage,
    F: FnMut(&mut S::Item) -> bool,
{
    fn drop(&mut self) {
        if !self.panicking {
            self.for_each(drop);
        }
    }
}

impl<S, F> FusedIterator for DrainFilter<'_, S, F>
where
    S: ?Sized + Storage,
    F: FnMut(&mut S::Item) -> bool,
{
}
impl<S, F> Iterator for DrainFilter<'_, S, F>
where
    S: ?Sized + Storage,
    F: FnMut(&mut S::Item) -> bool,
{
    type Item = S::Item;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.raw.is_empty() {
                break None
            }

            unsafe {
                let value = self.raw.front_mut();

                let on_drop = SetOnDrop(&mut self.panicking);
                let do_take = (self.filter)(value);
                core::mem::forget(on_drop);

                if do_take {
                    break Some(self.raw.take_front())
                }
                self.raw.skip_front();
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.raw.len();
        (0, Some(len))
    }
}

impl<S, F> DoubleEndedIterator for DrainFilter<'_, S, F>
where
    S: ?Sized + Storage,
    F: FnMut(&mut S::Item) -> bool,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        loop {
            if self.raw.is_empty() {
                break None
            }

            unsafe {
                let value = self.raw.back_mut();

                let on_drop = SetOnDrop(&mut self.panicking);
                let do_take = (self.filter)(value);
                core::mem::forget(on_drop);

                if do_take {
                    break Some(self.raw.take_back())
                }
                self.raw.skip_back();
            }
        }
    }
}
