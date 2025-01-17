use crate::{SimpleVec, Storage, SliceVec};

pub trait Extension<T> {
    unsafe fn extend_from_slice(&mut self, slice: &[T]);

    unsafe fn grow(&mut self, additional: usize, value: T);
}

fn clone_extend_from_slice<S: ?Sized + Storage>(vec: &mut SimpleVec<S>, slice: &[S::Item])
where
    S::Item: Clone,
{
    let spare = vec.spare_capacity_mut();
    let mut writer = unsafe { SliceVec::new(spare) };

    for value in slice.iter().cloned() {
        // Safety
        //
        // `clone_extend_from_slice` is only called from `Extension::extend_from_slice`
        // which has the pre-condition that there must be at least enough remaining capacity
        // for the slice. So it is safe to write the contents of the slice
        unsafe {
            writer.push_unchecked(value);
        }
    }

    unsafe {
        let writer = core::mem::ManuallyDrop::new(writer);
        let len = writer.len() + vec.len();
        vec.set_len_unchecked(len);
    }
}

fn clone_grow<S: ?Sized + Storage>(vec: &mut SimpleVec<S>, additional: usize, value: S::Item)
where
    S::Item: Clone,
{
    let spare = vec.spare_capacity_mut();
    let mut writer = unsafe { SliceVec::new(spare) };

    if additional != 0 {
        // Safety
        //
        // `clone_extend_from_slice` is only called from `Extension::extend_from_slice`
        // which has the pre-condition that there must be at least enough remaining capacity
        // for the slice. So it is safe to write the contents of the slice
        unsafe {
            for _ in 1..additional {
                writer.push_unchecked(value.clone());
            }

            writer.push_unchecked(value);
        }
    }

    unsafe {
        let writer = core::mem::ManuallyDrop::new(writer);
        let len = writer.len() + vec.len();
        vec.set_len_unchecked(len);
    }
}

impl<S: ?Sized + Storage> Extension<S::Item> for SimpleVec<S>
where
    S::Item: Clone,
{
    #[cfg(feature = "nightly")]
    default unsafe fn extend_from_slice(&mut self, slice: &[S::Item]) { clone_extend_from_slice(self, slice) }

    #[cfg(not(feature = "nightly"))]
    unsafe fn extend_from_slice(&mut self, slice: &[S::Item]) { clone_extend_from_slice(self, slice) }

    #[cfg(feature = "nightly")]
    default unsafe fn grow(&mut self, additional: usize, value: S::Item) { clone_grow(self, additional, value) }

    #[cfg(not(feature = "nightly"))]
    unsafe fn grow(&mut self, additional: usize, value: S::Item) { clone_grow(self, additional, value) }
}

#[cfg(feature = "nightly")]
impl<S: ?Sized + Storage> Extension<S::Item> for SimpleVec<S>
where
    S::Item: Copy,
{
    unsafe fn extend_from_slice(&mut self, slice: &[S::Item]) {
        // Safety
        //
        // * `Extension::extend_from_slice`'s precondition ensure that
        //   there is enough capacity for `slice`
        // * `T: Copy`, so there is nothing to drop
        unsafe { self.extend_from_slice_unchecked(slice) }
    }

    default unsafe fn grow(&mut self, additional: usize, value: S::Item) {
        // Safety
        //
        // * `Extension::grow`'s precondition ensure that
        //   there is enough capacity for `additional` elements
        let len = self.len();
        unsafe {
            self.set_len_unchecked(len.wrapping_add(additional));
        }
        let mut ptr = self.as_mut().as_mut_ptr();

        for _ in 0..additional {
            unsafe {
                ptr.write(value);
                ptr = ptr.add(1);
            }
        }
    }
}
