#![feature(implication_op)]
//@run-pass

fn main() {
    assert!(false ==> false);
    assert!(false ==> true);
    assert!(!(true ==> false));
    assert!(true ==> true);

    let mut check = true;

    _ = false ==> (check = false, false).1;
    assert!(check);
    _ = true ==> (check = false, true).1;
    assert!(!check);
}
