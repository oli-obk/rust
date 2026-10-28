//@ compile-flags: --crate-type=lib -Znext-solver
//@ aux-build: drop_struct.rs

#![feature(ettsd)]

extern crate core;
extern crate drop_struct;

use core::instantiate_optional;

macro_rules! opt {
    ($name:ident, $t:ty) => {
        instantiate_optional!($name, $t);
        impl $name {
            fn take(&mut self) -> Option<$t> {
                if self.discr {
                    self.discr = false;
                    Some(unsafe { self.data.assume_init_read() })
                } else {
                    None
                }
            }
        }
    };
}

opt!(OptU32, u32);

opt!(OptLayout, std::alloc::Layout);

opt!(OptF32, f32);

opt!(OptBool, bool);

opt!(OptFoo, drop_struct::Foo);

fn thing(x: &OptU32) {
    let y = *x;
    let y = x.clone();
}

fn foo(x: &drop_struct::Foo) {
    let y = *x;
    //~^ ERROR: cannot move out of `*x`
}
