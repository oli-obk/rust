pub struct Foo {
    x: *const (),
}

impl Drop for Foo {
    fn drop(&mut self) {}
}
