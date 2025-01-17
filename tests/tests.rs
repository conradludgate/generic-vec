#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(all(feature = "alloc", not(feature = "std")))]
extern crate alloc as std;

use cl_generic_vec::{ArrayVec, GenericVec};
use core::mem::MaybeUninit;
#[cfg(feature = "alloc")]
use mockalloc::Mockalloc;
#[cfg(feature = "std")]
use std::alloc::System;

#[global_allocator]
#[cfg(feature = "std")]
static ALLOCATOR: Mockalloc<System> = Mockalloc(System);

#[global_allocator]
#[cfg(all(feature = "alloc", not(feature = "std")))]
static ALLOCATOR: Mockalloc<static_alloc::Bump<[u8; 1 << 22]>> = Mockalloc(static_alloc::Bump::new([0; 1 << 22]));

#[cfg(feature = "alloc")]
macro_rules! S {
    ([$($e:expr),* $(,)?]) => {
        [$({
            let x = $e;
            crate::to_string::to_string(&x)
        }),*]
    };
    ($l:expr) => {
        {
            let x = $l;
            crate::to_string::to_string(&x)
        }
    };
}

#[cfg(feature = "alloc")]
mod to_string {
    pub trait TestToString: std::string::ToString {}
    pub fn to_string<T: TestToString>(t: &T) -> std::string::String { t.to_string() }

    impl TestToString for i32 {}
    impl TestToString for &i32 {}
    impl TestToString for &str {}
    impl TestToString for &&str {}
}

macro_rules! imp_make_tests_files {
    ($(#[$meta:meta])*mod $mod:ident {
        $($ident:ident),* $(,)?
    }) => {
        $(#[$meta])*
        mod $mod {
            $(
                mod $ident {
                    include!(concat!("template/", stringify!($mod), "/", stringify!($ident), ".rs"));
                }
            )*
        }
    };
}

macro_rules! make_tests_files {
    () => {
        make_tests_files! { copy_only }
        imp_make_tests_files! {
            #[cfg(feature = "alloc")]
            mod owned { simple, into_iter, cursor, drain, splice, vec_ops }
        }
    };
    (copy_only) => {
        imp_make_tests_files! {
            mod copy { simple, into_iter, cursor, drain, splice, vec_ops }
        }
    };
}

mod array_vec {
    macro_rules! new_vec {
        ($vec:pat, max($len:expr)) => {
            #[cfg(feature = "alloc")]
            let _bump = std::boxed::Box::new(1);
            let $vec = cl_generic_vec::ArrayVec::<_, $len>::new();
        };
    }

    make_tests_files!();
}

mod slice_vec {
    macro_rules! new_vec {
        ($vec:pat, max($len:expr)) => {
            #[cfg(feature = "alloc")]
            let _bump = std::boxed::Box::new(1);
            let mut buf = cl_generic_vec::uninit_array::<_, $len>();
            let $vec = unsafe { cl_generic_vec::SliceVec::new(&mut buf) };
        };
    }

    make_tests_files!();
}

#[cfg(feature = "alloc")]
mod heap_vec {
    macro_rules! new_vec {
        ($vec:pat, max($len:expr)) => {
            let _bump = std::boxed::Box::new(1);
            let $vec = cl_generic_vec::HeapVec::new();
        };
    }

    make_tests_files!();
}

#[test]
fn unsized_slice_vec() {
    let mut array_vec = ArrayVec::<i32, 16>::new();

    array_vec.push(1);
    assert_eq!(array_vec.len(), 1);
    assert_eq!(array_vec.capacity(), 16);
    assert_eq!(array_vec, [1]);

    let slice_vec: &mut GenericVec<i32, [MaybeUninit<_>]> = &mut array_vec;

    slice_vec.push(2);
    assert_eq!(slice_vec.len(), 2);
    assert_eq!(slice_vec.capacity(), 16);
    assert_eq!(*slice_vec, [1, 2]);
}
