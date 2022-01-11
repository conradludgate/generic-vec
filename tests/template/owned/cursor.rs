// use std::string::ToString;

#[mockalloc::test]
fn raw_drain_front() {
    new_vec!(mut vec, max(8));

    vec.push(S!("0"));
    vec.push(S!("2"));
    vec.push(S!("1"));

    {
        let mut drain = vec.cursor(..);

        assert_eq!(drain.take_front(), "0");
        assert_eq!(drain.take_front(), "2");
        assert_eq!(drain.take_front(), "1");
    }

    assert_eq!(vec, []);

    vec.push(S!("0"));
    vec.push(S!("2"));
    vec.push(S!("1"));

    {
        let mut drain = vec.cursor(..);

        assert_eq!(drain.take_front(), "0");
        assert_eq!(drain.take_front(), "2");
    }

    assert_eq!(vec, S!([1]));

    vec.push(S!("0"));
    vec.push(S!("2"));
    vec.push(S!("1"));

    {
        let mut drain = vec.cursor(..);

        assert_eq!(drain.take_front(), "1");
        drain.skip_front();
        assert_eq!(drain.take_front(), "2");
    }

    assert_eq!(vec, S!([0, 1]));

    vec.push(S!("0"));
    vec.push(S!("2"));
    vec.push(S!("1"));

    {
        let mut drain = vec.cursor(..);

        assert_eq!(drain.take_front(), "0");
    }

    assert_eq!(vec, S!([1, 0, 2, 1]))
}

#[mockalloc::test]
fn raw_drain_back() {
    new_vec!(mut vec, max(8));

    vec.push(S!("0"));
    vec.push(S!("2"));
    vec.push(S!("1"));

    {
        let mut drain = vec.cursor(..);

        assert_eq!(drain.take_back(), "1");
        assert_eq!(drain.take_back(), "2");
        assert_eq!(drain.take_back(), "0");
    }

    assert_eq!(vec, []);

    vec.push(S!("0"));
    vec.push(S!("2"));
    vec.push(S!("1"));

    {
        let mut drain = vec.cursor(..);

        assert_eq!(drain.take_back(), "1");
        assert_eq!(drain.take_back(), "2");
    }

    assert_eq!(vec, S!([0]));

    vec.push(S!("0"));
    vec.push(S!("2"));
    vec.push(S!("1"));

    {
        let mut drain = vec.cursor(..);

        assert_eq!(drain.take_back(), "1");
        drain.skip_back();
        assert_eq!(drain.take_back(), "0");
    }

    assert_eq!(vec, S!([0, 2]));

    vec.push(S!("0"));
    vec.push(S!("2"));
    vec.push(S!("1"));

    {
        let mut drain = vec.cursor(..);

        assert_eq!(drain.take_back(), "1");
    }

    assert_eq!(vec, S!([0, 2, 0, 2]))
}
