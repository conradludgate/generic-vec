use crate::{
    raw::{Storage, StorageWithCapacity},
    uninit_array,
};
use std::mem::MaybeUninit;

use super::{AllocError, AllocResult};

unsafe impl<T, const N: usize> StorageWithCapacity for [MaybeUninit<T>; N] {
    fn with_capacity(capacity: usize) -> Self {
        if capacity > N {
            crate::raw::capacity::fixed_capacity_reserve_error(N, capacity)
        }

        uninit_array()
    }

    #[inline]
    #[doc(hidden)]
    #[allow(non_snake_case)]
    fn __with_capacity__const_capacity_checked(capacity: usize, old_capacity: Option<usize>) -> Self {
        match old_capacity {
            Some(old_capacity) if old_capacity <= N => uninit_array(),
            _ => Self::with_capacity(capacity),
        }
    }
}

unsafe impl<T, const N: usize> Storage for [MaybeUninit<T>; N] {
    type Item = T;

    #[doc(hidden)]
    const CONST_CAPACITY: Option<usize> = Some(N);

    fn reserve(&mut self, capacity: usize) {
        if capacity > N {
            crate::raw::capacity::fixed_capacity_reserve_error(N, capacity)
        }
    }

    fn try_reserve(&mut self, capacity: usize) -> AllocResult {
        if capacity <= N {
            Ok(())
        } else {
            Err(AllocError)
        }
    }
}
