#[mockalloc::test]
fn cursor_front() {
    new_vec!(mut vec, max(8));

    vec.push(0);
    vec.push(2);
    vec.push(1);

    {
        let mut drain = vec.cursor(..);

        assert_eq!(drain.take_front(), 0);
        assert_eq!(drain.take_front(), 2);
        assert_eq!(drain.take_front(), 1);
    }

    assert_eq!(vec, []);

    vec.push(0);
    vec.push(2);
    vec.push(1);

    {
        let mut drain = vec.cursor(..);

        assert_eq!(drain.take_front(), 0);
        assert_eq!(drain.take_front(), 2);
    }

    assert_eq!(vec, [1]);

    vec.push(0);
    vec.push(2);
    vec.push(1);

    {
        let mut drain = vec.cursor(..);

        assert_eq!(drain.take_front(), 1);
        drain.skip_front();
        assert_eq!(drain.take_front(), 2);
    }

    assert_eq!(vec, [0, 1]);

    vec.push(0);
    vec.push(2);
    vec.push(1);

    {
        let mut drain = vec.cursor(..);

        assert_eq!(drain.take_front(), 0);
    }

    assert_eq!(vec, [1, 0, 2, 1])
}

#[mockalloc::test]
fn cursor_back() {
    new_vec!(mut vec, max(8));

    vec.push(0);
    vec.push(2);
    vec.push(1);

    {
        let mut drain = vec.cursor(..);

        assert_eq!(drain.take_back(), 1);
        assert_eq!(drain.take_back(), 2);
        assert_eq!(drain.take_back(), 0);
    }

    assert_eq!(vec, []);

    vec.push(0);
    vec.push(2);
    vec.push(1);

    {
        let mut drain = vec.cursor(..);

        assert_eq!(drain.take_back(), 1);
        assert_eq!(drain.take_back(), 2);
    }

    assert_eq!(vec, [0]);

    vec.push(0);
    vec.push(2);
    vec.push(1);

    {
        let mut drain = vec.cursor(..);

        assert_eq!(drain.take_back(), 1);
        drain.skip_back();
        assert_eq!(drain.take_back(), 0);
    }

    assert_eq!(vec, [0, 2]);

    vec.push(0);
    vec.push(2);
    vec.push(1);

    {
        let mut drain = vec.cursor(..);

        assert_eq!(drain.take_back(), 1);
    }

    assert_eq!(vec, [0, 2, 0, 2])
}

#[mockalloc::test]
fn drain() {
    new_vec!(mut vec, max(8));

    vec.extend([0, 1, 2, 3, 4, 5, 6, 7].iter().copied());

    assert_eq!(vec, [0, 1, 2, 3, 4, 5, 6, 7]);

    vec.drain(4..7);

    assert_eq!(vec, [0, 1, 2, 3, 7]);

    assert!(vec.drain(1..3).eq([1, 2].iter().copied()));

    assert_eq!(vec, [0, 3, 7]);
}

#[mockalloc::test]
fn drain_filter() {
    new_vec!(mut vec, max(8));

    vec.extend([0, 1, 2, 3, 4, 5, 6, 7].iter().copied());

    vec.drain_filter(.., |&mut x| x % 2 == 0);

    assert_eq!(vec, [1, 3, 5, 7]);

    assert!(vec.drain_filter(.., |&mut x| x % 3 == 0).eq([3].iter().copied()));

    assert_eq!(vec, [1, 5, 7]);
}
