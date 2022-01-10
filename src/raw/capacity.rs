#[cold]
#[inline(never)]
pub(in crate::raw) fn fixed_capacity_reserve_error(capacity: usize, new_capacity: usize) -> ! {
    panic!(
        "Tried to reserve {}, but used a fixed capacity storage of {}",
        new_capacity, capacity
    )
}
