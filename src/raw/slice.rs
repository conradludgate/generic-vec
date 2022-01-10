use crate::raw::{capacity::fixed_capacity_reserve_error, Storage};

use core::mem::MaybeUninit;

use super::{AllocError, AllocResult};

unsafe impl<T> Storage for [MaybeUninit<T>] {
    type Item = T;

    fn reserve(&mut self, new_capacity: usize) {
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
