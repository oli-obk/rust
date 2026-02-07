#![feature(try_as_dyn)]

use std::any::try_as_dyn;

type Payload = &'static i32;

trait Convert<T> {
    fn convert(&self) -> &T;
}

impl<T> Convert<T> for T {
    fn convert(&self) -> &T {
        self
    }
}

const _: () = {
    let payload: &Payload = &&1;
    let thing: &Payload = &*payload;
    let convert: &dyn Convert<&'static Payload> = try_as_dyn(&thing).unwrap();
    //~^ ERROR: `Option::unwrap()` on a `None` value
};

fn main() {}
