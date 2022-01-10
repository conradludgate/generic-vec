use crate::raw::{
    capacity::{capacity, fixed_capacity_reserve_error, Round},
    Storage,
};

use core::mem::{size_of, MaybeUninit};

use super::{AllocError, AllocResult};

unsafe impl<T> Storage for [MaybeUninit<T>] {
    type Item = T;

    fn reserve(&mut self, new_capacity: usize) {
        let new_capacity = capacity(new_capacity, size_of::<T>(), size_of::<T>(), Round::Up);
        if new_capacity > self.len() {
            fixed_capacity_reserve_error(self.len(), new_capacity)
        }
    }

    fn try_reserve(&mut self, capacity: usize) -> AllocResult {
        if capacity <= self.len() {
            Ok(())
        } else {
            Err(AllocError)
        }
    }
}
