macro_rules! implication {
    ($x:expr) => {
        0
    };
    ($a:expr ==> $b:expr) => {
        //~^ ERROR: is followed by `==`, which is not allowed for `expr` fragments
        1
    };
    ($x:expr) => {
        2
    };
}

const _: () = {
    assert!(implication!(true ==> false) == 0);
};

fn main() {}
