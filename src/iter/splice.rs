use crate::{iter::RawCursor, Storage};

/// This struct is created by [`GenericVec::splice`](crate::GenericVec::splice).
/// See its documentation for more.
pub struct Splice<'a, S, I>
where
    S: ?Sized + Storage,
    I: Iterator<Item = S::Item>,
{
    raw: RawCursor<'a, S>,
    replace_with: I,
}

impl<'a, S: ?Sized + Storage, I: Iterator<Item = S::Item>> Splice<'a, S, I> {
    pub(crate) fn new(raw: RawCursor<'a, S>, replace_with: I) -> Self { Self { raw, replace_with } }
}

impl<S: ?Sized + Storage, I: Iterator<Item = S::Item>> Drop for Splice<'_, S, I> {
    fn drop(&mut self) {
        unsafe {
            self.raw.drop_n_front(self.raw.len());
        }

        let Self { raw, replace_with } = self;

        if raw.at_back_of_vec() {
            self.raw.finish();
            unsafe { self.raw.vec_mut().extend(replace_with) }
            return
        }

        while !raw.is_write_empty() {
            match replace_with.next() {
                Some(value) => unsafe { raw.write_front(value) },
                None => return,
            }
        }

        #[cfg(not(feature = "alloc"))]
        {
            const CAPACITY: usize = 16;

            let mut buffer = crate::uninit_array::<_, CAPACITY>();
            let mut buffer = crate::SliceVec::new(&mut buffer);

            replace_with.for_each(|item| unsafe {
                buffer.push_unchecked(item);

                if buffer.is_full() {
                    unsafe {
                        raw.reserve(buffer.len());
                        raw.write_slice_front(&buffer);
                        buffer.set_len_unchecked(0);
                    }
                }
            });

            unsafe {
                raw.reserve(buffer.len());
                raw.write_slice_front(&buffer);
                core::mem::forget(buffer);
            }
        }

        #[cfg(feature = "alloc")]
        {
            let mut temp: std::vec::Vec<S::Item> = replace_with.collect();

            unsafe {
                raw.reserve(temp.len());
                raw.write_slice_front(&temp);
                temp.set_len(0);
            }
        }
    }
}

impl<S: ?Sized + Storage, I: Iterator<Item = S::Item>> ExactSizeIterator for Splice<'_, S, I> {}

impl<'a, S: ?Sized + Storage, I: Iterator<Item = S::Item>> Iterator for Splice<'a, S, I> {
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        if self.raw.is_empty() {
            None
        } else {
            Some(unsafe { self.raw.take_front() })
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let size = self.raw.len();
        (size, Some(size))
    }
}

impl<'a, S: ?Sized + Storage, I: Iterator<Item = S::Item>> DoubleEndedIterator for Splice<'a, S, I> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.raw.is_empty() {
            None
        } else {
            Some(unsafe { self.raw.take_back() })
        }
    }
}
