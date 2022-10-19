#![deny(unreachable_pub)]

// check-pass

pub use crate::builder::Foo;

mod builder {
    pub struct Foo(pub(crate) ());
}

fn main() {}
