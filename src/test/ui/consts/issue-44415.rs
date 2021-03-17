#![feature(core_intrinsics)]

use std::intrinsics;

struct Foo {
    bytes: [u8; unsafe { intrinsics::size_of::<Foo>() }],
    //~^ ERROR cycle detected when evaluating constant for use in types
    x: usize,
}

fn main() {}
