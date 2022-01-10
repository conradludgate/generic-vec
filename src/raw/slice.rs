use crate::raw::{
    capacity::{capacity, fixed_capacity_reserve_error, Round},
    Storage,
};

use core::mem::{align_of, size_of, MaybeUninit};

unsafe impl<T, U> Storage<U> for [MaybeUninit<T>] {
    const IS_ALIGNED: bool = align_of::<T>() >= align_of::<U>();

    fn capacity(&self) -> usize { capacity(self.len(), size_of::<T>(), size_of::<U>(), Round::Down) }

    fn as_ptr(&self) -> *const U { self.as_ptr().cast() }

    fn as_mut_ptr(&mut self) -> *mut U { self.as_mut_ptr().cast() }

    fn reserve(&mut self, new_capacity: usize) {
        let new_capacity = capacity(new_capacity, size_of::<U>(), size_of::<T>(), Round::Up);
        if new_capacity > self.len() {
            fixed_capacity_reserve_error(self.len(), new_capacity)
        }
    }

    fn try_reserve(&mut self, capacity: usize) -> bool { capacity <= Storage::<U>::capacity(self) }
}
