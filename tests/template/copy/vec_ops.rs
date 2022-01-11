use cl_generic_vec::SliceVec;

#[mockalloc::test]
fn split_off() {
    new_vec!(mut vec, max(8));
    vec.extend(0..8);
    let mut other = cl_generic_vec::uninit_array::<_, 4>();
    let mut other = unsafe { SliceVec::new(&mut other) };
    vec.split_off_into(4, &mut other);
    assert_eq!(vec, [0, 1, 2, 3]);
    assert_eq!(other, [4, 5, 6, 7]);
}

#[mockalloc::test]
fn grow() {
    new_vec!(mut vec, max(4));
    vec.grow(4, 0);
    assert_eq!(vec, [0; 4]);
}
