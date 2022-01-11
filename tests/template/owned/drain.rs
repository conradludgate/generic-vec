#[mockalloc::test]
fn drain() {
    new_vec!(mut vec, max(8));

    vec.extend([0, 1, 2, 3, 4, 5, 6, 7].iter().map(|x| S!(x)));

    assert_eq!(vec, S!([0, 1, 2, 3, 4, 5, 6, 7]));

    vec.drain(4..7);

    assert_eq!(vec, S!([0, 1, 2, 3, 7]));

    assert!(vec.drain(1..3).rev().eq([2, 1].iter().map(|x| S!(x))));

    assert_eq!(vec, S!([0, 3, 7]));
}

#[mockalloc::test]
fn drain_filter() {
    new_vec!(mut vec, max(8));

    vec.extend(
        ["0", "00", "000", "0000", "00000", "000000", "0000000", "00000000"]
            .iter()
            .map(|x| S!(x)),
    );

    vec.drain_filter(.., |x| x.len() % 2 == 0);

    assert_eq!(vec, S!(["0", "000", "00000", "0000000",]));

    assert!(vec.drain_filter(.., |x| x.len() % 3 == 0).eq(Some(S!("000"))));

    assert_eq!(vec, S!(["0", "00000", "0000000",]));
}
