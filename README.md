[![Crates.io](https://img.shields.io/crates/v/generic-vec.svg)](https://crates.io/crates/generic-vec)
[![Docs.rs](https://docs.rs/generic-vec/badge.svg)](https://docs.rs/generic-vec)
[![Workflow Status](https://github.com/rustyyato/generic-vec/workflows/main/badge.svg)](https://github.com/rustyyato/generic-vec/actions?query=workflow%3A%22main%22)
![Maintenance](https://img.shields.io/badge/maintenance-activly--developed-brightgreen.svg)

# generic-vec

A vector that can store items anywhere: in slices, arrays, or the heap!

`GenericVec` has complete parity with `Vec`, and even provides some features
that are only in `nightly` on `std` (like `GenericVec::drain_filter`), or a more permissive
interface like `GenericVec::retain`. In fact, you can trivially convert a `Vec` to a
`HeapVec` and back!

This crate is `no_std` compatible, just turn off all default features.

## Features

* `std` (default) - enables you to use an allocator, and
* `alloc` - enables you to use an allocator, for heap allocated storages
    (like `Vec`)
* `nightly` - enables you to use array (`[T; N]`) based storages

## Basic Usage

#### `SliceVec`

`SliceVec` stores an uninit slice buffer, and they store all of thier values in that buffer.

```rust
use cl_generic_vec::SliceVec;

let mut uninit_buffer = uninit_array::<_, 16>();
let mut slice_vec = SliceVec::new(&mut uninit_buffer);

assert!(slice_vec.is_empty());
slice_vec.push(10);
assert_eq!(slice_vec, [10]);
```

#### `ArrayVec`

`ArrayVec` is just like the slice versions, but since they own their data,
they can be freely moved around, unconstrained. You can also create
a new `ArrayVec` without passing in an existing buffer,
unlike the slice versions.

```rust
use cl_generic_vec::ArrayVec;

let mut array_vec = ArrayVec::<i32, 16>::new();

array_vec.push(10);
array_vec.push(20);
array_vec.push(30);

assert_eq!(array_vec, [10, 20, 30]);
```

### `alloc`

A `HeapVec` is just `Vec`, but built atop `GenericVec`,
meaning you get all the features of `GenericVec` for free! But this
requries either the `alloc` or `std` feature to be enabled.

```rust
use cl_generic_vec::{HeapVec, gvec};
let mut vec: HeapVec<u32> = gvec![1, 2, 3, 4];
assert_eq!(vec.capacity(), 4);
vec.extend(&[5, 6, 7, 8]);

assert_eq!(vec, [1, 2, 3, 4, 5, 6, 7, 8]);

vec.try_push(5).expect_err("Tried to push past capacity!");
```

### `nightly`

On `nightly`
* a number of optimizations are enabled
* some diagnostics become better

Note on the documentation: if the feature exists on `Vec`, then the documentation
is either exactly the same as `Vec` or slightly adapted to better fit `GenericVec`

Note on implementation: large parts of the implementation came straight from `Vec`
so thanks for the amazing reference `std`!

Current version: 0.1.2

License: MIT/Apache-2.0
