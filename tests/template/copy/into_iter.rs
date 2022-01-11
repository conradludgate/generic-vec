#[mockalloc::test]
pub fn into_iter() {
    new_vec!(mut vec, max(8));
    vec.extend(0..8);

    assert!((0..8).eq(vec));
}
