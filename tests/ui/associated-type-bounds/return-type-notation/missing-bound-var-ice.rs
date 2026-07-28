//@ needs-rustc-debug-assertions
//! This test used to ICE #133613

#![feature(return_type_notation)]

struct Wrapper();

trait IntFactory {
    fn stream(&self) -> impl IntFactory<stream(..): IntFactory<stream(..): Send>>;
}

fn main() {}
