#[mockalloc::test]
pub fn simple() {
    new_vec!(mut vec, max(8));

    assert_eq!(vec.len(), 0);
    assert_eq!(*vec.push(S!("0")), "0");
    assert_eq!(*vec.push(S!("2")), "2");
    assert_eq!(*vec.push(S!("1")), "1");
    assert_eq!(vec, S!([0, 2, 1]));
    assert_eq!(vec.pop(), "1");
    assert_eq!(vec, S!([0, 2]));
    assert_eq!(*vec.insert(1, S!("9")), "9");
    assert_eq!(*vec.insert(2, S!("8")), "8");
    assert_eq!(*vec.insert(3, S!("7")), "7");
    assert_eq!(vec, S!([0, 9, 8, 7, 2]));
    assert_eq!(vec.remove(2), "8");
    assert_eq!(vec.remove(2), "7");
    assert_eq!(vec, S!([0, 9, 2]));
    assert_eq!(vec.swap_remove(0), "0");
    assert_eq!(vec, S!([2, 9]));
}

#[mockalloc::test]
#[cfg(feature = "nightly")]
pub fn array_ops() {
    new_vec!(mut vec, max(8));

    assert_eq!(vec.len(), 0);
    assert_eq!(*vec.push_array(S!([0, 2, 1])), S!([0, 2, 1]));
    assert_eq!(vec, S!([0, 2, 1]));
    assert_eq!(vec.pop_array(), S!([1]));
    assert_eq!(vec, S!([0, 2]));
    assert_eq!(*vec.insert_array(1, S!([9, 8, 7])), S!([9, 8, 7]));
    assert_eq!(vec, S!([0, 9, 8, 7, 2]));
    assert_eq!(vec.remove_array(2), S!([8, 7]));
    assert_eq!(vec, S!([0, 9, 2]));
}
