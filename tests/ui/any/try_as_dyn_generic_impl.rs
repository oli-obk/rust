#![feature(try_as_dyn)]
//@ check-pass

use std::any::try_as_dyn;

struct Thing<T>(T);
trait Trait {}
impl<T> Trait for Thing<T> {}

const _: () = {
    let thing = Thing(1);
    assert!(try_as_dyn::<_, dyn Trait>(&thing).is_none());
};

struct Thing2<T>(T);
impl<T: std::fmt::Debug> Trait for Thing2<T> {}

const _: () = {
    let thing = Thing2(1);
    assert!(try_as_dyn::<_, dyn Trait>(&thing).is_none());
};

trait Trait2 {}
impl<'a, 'b> Trait2 for &'a &'b () {}

struct Thing3<T>(T);
impl<T: Trait2> Trait for Thing3<T> {}

const _: () = {
    let thing = Thing3(&&());
    assert!(try_as_dyn::<_, dyn Trait2>(&thing).is_none());
};

fn main() {}
