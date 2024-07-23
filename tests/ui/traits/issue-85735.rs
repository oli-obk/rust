// Regression test for the invalid suggestion in #85735 (the
// underlying issue #21974 still exists here).

trait Foo {}
impl<'a, 'b, T> Foo for T
where
    T: FnMut(&'a ()),
    T: FnMut(&'b ()),
    //~^ ERROR: type annotations needed
{
}

fn main() {}
