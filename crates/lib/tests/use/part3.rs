use super::*;

#[test]
#[ignore = "we don't hermetically evaluate @extend"]
fn use_module_with_extend() {
    let mut fs = TestFs::new();

    fs.add_file(
        "_a.scss",
        r#"
        a {
            @extend b;
        }
    "#,
    );

    let input = r#"
        @use "a";
        b {
            color: red;
        }
    "#;

    assert_err!(
        input,
        "Error: The target selector was not found.",
        fugo_sass::Options::default().fs(&fs)
    );
}

// todo: refactor these tests to use testfs where possible

#[test]
fn extend_reaches_upstream_module() {
    let mut fs = TestFs::new();

    fs.add_file("_up.scss", "%a {color: red}\n.c {color: blue}\n");

    let input = "@use \"up\";\n.b {@extend %a; @extend .c;}\n";

    assert_eq!(
        ".b {\n  color: red;\n}\n\n.c, .b {\n  color: blue;\n}\n",
        &fugo_sass::from_string(input.to_string(), &fugo_sass::Options::default().fs(&fs))
            .expect(input)
    );
}

#[test]
fn private_placeholder_is_not_extended_across_modules() {
    let mut fs = TestFs::new();

    fs.add_file("_up.scss", "%-a {color: red}\n");

    let input = "@use \"up\";\n.b {@extend %-a;}\n";

    assert_eq!(
        "Error: The target selector was not found.",
        fugo_sass::from_string(input.to_string(), &fugo_sass::Options::default().fs(&fs))
            .unwrap_err()
            .to_string()
            .lines()
            .next()
            .unwrap()
    );
}

#[test]
fn optional_extend_without_target_across_modules() {
    let mut fs = TestFs::new();

    fs.add_file("_up.scss", ".a {color: red}\n");

    let input = "@use \"up\";\n.b {@extend .c !optional;}\n";

    assert_eq!(
        ".a {\n  color: red;\n}\n",
        &fugo_sass::from_string(input.to_string(), &fugo_sass::Options::default().fs(&fs))
            .expect(input)
    );
}
