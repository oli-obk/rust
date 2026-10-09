//@ compile-flags: --crate-type=lib -Znext-solver
//@ check-pass

#![feature(ettsd)]

extern crate core;

use core::instantiate_optional;

instantiate_optional!(OptU32, u32);

instantiate_optional!(OptLayout, std::alloc::Layout);

instantiate_optional!(OptF32, f32);

instantiate_optional!(OptBool, bool);

fn thing(x: &OptU32) {
    let y = *x;
    let y = x.clone();
}
