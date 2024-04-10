fn main() {
    assert!(false ==> false);
    //~^ ERROR found `==>`
    assert!(false ==> true);
    //~^ ERROR found `==>`
    assert!(!(true ==> false));
    //~^ ERROR found `==>`
    assert!(true ==> true);
    //~^ ERROR found `==>`
}
