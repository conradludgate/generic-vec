use cl_generic_vec::SliceVec;

#[mockalloc::test]
fn split_off() {
    new_vec!(mut vec, max(8));
    vec.extend((0..8).map(|x| S!(x)));
    let mut other = cl_generic_vec::uninit_array::<_, 4>();
    let mut other = unsafe { SliceVec::new(&mut other) };
    vec.split_off_into(4, &mut other);
    assert_eq!(vec, S!([0, 1, 2, 3]));
    assert_eq!(other, S!([4, 5, 6, 7]));
}

#[mockalloc::test]
fn consume_extend() {
    new_vec!(mut vec, max(4));
    let mut other = cl_generic_vec::uninit_array::<_, 4>();
    let mut other = unsafe { SliceVec::new(&mut other) };
    other.extend((0..4).map(|x| S!(x)));
    other.split_off_into(0, &mut vec);
    assert_eq!(vec, S!([0, 1, 2, 3]));
    assert_eq!(other, []);
}

#[mockalloc::test]
fn grow() {
    new_vec!(mut vec, max(4));
    vec.grow(4, S!(0));
    assert_eq!(vec, [S!(0), S!(0), S!(0), S!(0)]);
}
