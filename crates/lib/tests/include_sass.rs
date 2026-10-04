#[cfg(feature = "macro")]
#[test]
fn basic() {
    let css: &str = fugo_sass::include!("./input.scss");

    assert_eq!(css, "a{color:red}");
}
