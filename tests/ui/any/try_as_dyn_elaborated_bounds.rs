//@ revisions: next old
//@[next] compile-flags: -Znext-solver

#![feature(try_as_dyn)]

use std::any::try_as_dyn;

trait Trait: 'static {}
trait Other {}
struct Foo<T>(T);

// This impl should not be visible, as it has a `T: 'static` bound
impl<T: Trait> Other for Foo<T> {}

const _: () = {
    let foo = Foo(());
    // TODO
    assert!(try_as_dyn::<Foo<()>, dyn Other>(&foo).is_some());
    //~^ ERROR: assertion failed
};

fn main() {}
