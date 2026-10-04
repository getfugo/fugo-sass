use super::*;

#[test]
fn null_fs_cannot_import() {
    let input = "@import \"__foo\";";
    tempfile!("__foo.scss", "");
    match fugo_sass::from_string(
        input.to_string(),
        &fugo_sass::Options::default().fs(&fugo_sass::NullFs),
    ) {
        Err(e)
            if e.to_string()
                .starts_with("Error: Can't find stylesheet to import.\n") => {}
        Ok(..) => panic!("did not fail"),
        Err(e) => panic!("failed in the wrong way: {}", e),
    }
}

#[test]
fn imports_variable() {
    let mut fs = TestFs::new();

    fs.add_file("a.scss", r#"$a: red;"#);

    let input = r#"
        @import "a";
        a {
            color: $a;
        }
    "#;

    assert_eq!(
        "a {\n  color: red;\n}\n",
        &fugo_sass::from_string(input.to_string(), &fugo_sass::Options::default().fs(&fs))
            .expect(input)
    );
}

#[test]
#[ignore = "we don't actually check if the semicolon exists"]
fn import_no_semicolon() {
    let input = "@import \"import_no_semicolon\"\na {\n color: $a;\n}";
    tempfile!("import_no_semicolon", "$a: red;");

    let _ = input;
}

#[test]
fn import_no_quotes() {
    let input = "@import import_no_quotes";

    assert_err!("Error: Expected string.", input);
}

#[test]
fn single_quotes_import() {
    let mut fs = TestFs::new();

    fs.add_file("a.scss", r#"$a: red;"#);

    let input = r#"
        @import 'a';
        a {
            color: $a;
        }
    "#;

    assert_eq!(
        "a {\n  color: red;\n}\n",
        &fugo_sass::from_string(input.to_string(), &fugo_sass::Options::default().fs(&fs))
            .expect(input)
    );
}

#[test]
fn comma_separated_import() {
    let mut fs = TestFs::new();

    fs.add_file("a.scss", r#"$a: red"#);
    fs.add_file("b.scss", r#"p { color: blue; }"#);

    let input = r#"
        @import 'a', 'b';

        a {
            color: $a;
        }
    "#;

    assert_eq!(
        "p {\n  color: blue;\n}\n\na {\n  color: red;\n}\n",
        &fugo_sass::from_string(input.to_string(), &fugo_sass::Options::default().fs(&fs))
            .expect(input)
    );
}

#[test]
fn comma_separated_import_order() {
    let mut fs = TestFs::new();

    fs.add_file("a.scss", r#"p { color: red; }"#);
    fs.add_file("b.scss", r#"p { color: blue; }"#);

    let input = r#"
        @import "a", "b", url(third);
    "#;

    assert_eq!(
        "@import url(third);\np {\n  color: red;\n}\n\np {\n  color: blue;\n}\n",
        &fugo_sass::from_string(input.to_string(), &fugo_sass::Options::default().fs(&fs))
            .expect(input)
    );
}

#[test]
fn comma_separated_import_order_css() {
    let mut fs = TestFs::new();

    fs.add_file("a.css", r#"p { color: red; }"#);
    fs.add_file("b.css", r#"p { color: blue; }"#);

    let input = r#"
        @import "a.css", "b", url(third);
    "#;

    assert_eq!(
        "@import \"a.css\";\n@import url(third);\np {\n  color: blue;\n}\n",
        &fugo_sass::from_string(input.to_string(), &fugo_sass::Options::default().fs(&fs))
            .expect(input)
    );
}

#[test]
fn basic_load_path() {
    tempfile!(
        "basic_load_path__a.scss",
        "@import \"basic_load_path__b\";\na {\n color: $a;\n}",
        dir = "dir-basic_load_path__a"
    );
    tempfile!(
        "basic_load_path__b.scss",
        "$a: red;",
        dir = "dir-basic_load_path__b"
    );

    assert_eq!(
        "a {\n  color: red;\n}\n",
        fugo_sass::from_path(
            "dir-basic_load_path__a/basic_load_path__a.scss",
            &fugo_sass::Options::default()
                .load_path(std::path::Path::new("dir-basic_load_path__b"))
        )
        .unwrap()
    );
}

#[test]
fn load_path_same_directory() {
    tempfile!(
        "load_path_same_directory__a.scss",
        "@import \"dir-load_path_same_directory__a/load_path_same_directory__b\";\na {\n color: $a;\n}",
        dir = "dir-load_path_same_directory__a"
    );
    tempfile!(
        "load_path_same_directory__b.scss",
        "$a: red;",
        dir = "dir-load_path_same_directory__a"
    );

    assert_eq!(
        "a {\n  color: red;\n}\n",
        fugo_sass::from_path(
            "dir-load_path_same_directory__a/load_path_same_directory__a.scss",
            &fugo_sass::Options::default().load_path(std::path::Path::new("."))
        )
        .unwrap()
    );
}

#[test]
fn comma_separated_import_trailing() {
    let mut fs = TestFs::new();

    fs.add_file("a.scss", r#"p { color: red; }"#);
    fs.add_file("b.scss", r#"p { color: blue; }"#);

    let input = r#"
        @import "a", "b", url(third),,,,,,,,;
    "#;

    assert_err!("Error: Expected string.", input);
}

#[test]
fn finds_name_scss() {
    let mut fs = TestFs::new();

    fs.add_file("a.scss", r#"$a: red;"#);

    let input = r#"
        @import "a";
        a {
            color: $a;
        }
    "#;

    assert_eq!(
        "a {\n  color: red;\n}\n",
        &fugo_sass::from_string(input.to_string(), &fugo_sass::Options::default().fs(&fs))
            .expect(input)
    );
}

#[test]
fn finds_underscore_name_scss() {
    let mut fs = TestFs::new();
    fs.add_file("_a.scss", r#"$a: red;"#);

    let input = r#"
        @import "a";
        a {
            color: $a;
        }
    "#;

    assert_eq!(
        "a {\n  color: red;\n}\n",
        &fugo_sass::from_string(input.to_string(), &fugo_sass::Options::default().fs(&fs))
            .expect(input)
    );
}

#[test]
fn chained_imports() {
    let mut fs = TestFs::new();

    fs.add_file("a.scss", r#"@import "b";"#);
    fs.add_file("b.scss", r#"@import "c";"#);
    fs.add_file("c.scss", r#"$a: red;"#);

    let input = r#"
        @import "a";
        a {
            color: $a;
        }
    "#;

    assert_eq!(
        "a {\n  color: red;\n}\n",
        &fugo_sass::from_string(input.to_string(), &fugo_sass::Options::default().fs(&fs))
            .expect(input)
    );
}

#[test]
fn imports_plain_css() {
    let mut fs = TestFs::new();

    fs.add_file("a.css", r#"a { color: red; }"#);

    let input = r#"
        @import "a";
    "#;

    assert_eq!(
        "a {\n  color: red;\n}\n",
        &fugo_sass::from_string(input.to_string(), &fugo_sass::Options::default().fs(&fs))
            .expect(input)
    );
}

#[test]
fn imports_import_only_scss() {
    let mut fs = TestFs::new();

    fs.add_file("a.import.scss", r#"a { color: red; }"#);

    let input = r#"
        @import "a";
    "#;

    assert_eq!(
        "a {\n  color: red;\n}\n",
        &fugo_sass::from_string(input.to_string(), &fugo_sass::Options::default().fs(&fs))
            .expect(input)
    );
}

#[test]
fn imports_sass_file() {
    let mut fs = TestFs::new();

    fs.add_file("a.sass", "a\n\tcolor: red\n");

    let input = r#"
        @import "a";
    "#;

    assert_eq!(
        "a {\n  color: red;\n}\n",
        &fugo_sass::from_string(input.to_string(), &fugo_sass::Options::default().fs(&fs))
            .expect(input)
    );
}

#[test]
fn imports_absolute_scss() {
    let mut fs = TestFs::new();

    fs.add_file("/foo/a.scss", r#"a { color: red; }"#);

    let input = r#"
        @import "/foo/a";
    "#;

    assert_eq!(
        "a {\n  color: red;\n}\n",
        &fugo_sass::from_string(input.to_string(), &fugo_sass::Options::default().fs(&fs))
            .expect(input)
    );
}

#[test]
fn imports_same_file_twice() {
    let mut fs = TestFs::new();

    fs.add_file("a.scss", r#"a { color: red; }"#);

    let input = r#"
        @import "a";
        @import "a";
    "#;

    assert_eq!(
        "a {\n  color: red;\n}\n\na {\n  color: red;\n}\n",
        &fugo_sass::from_string(input.to_string(), &fugo_sass::Options::default().fs(&fs))
            .expect(input)
    );
}

#[test]
fn imports_same_file_thrice() {
    let mut fs = TestFs::new();

    fs.add_file("a.scss", r#"a { color: red; }"#);

    let input = r#"
        @import "a";
        @import "a";
        @import "a";
    "#;

    assert_eq!(
        "a {\n  color: red;\n}\n\na {\n  color: red;\n}\n\na {\n  color: red;\n}\n",
        &fugo_sass::from_string(input.to_string(), &fugo_sass::Options::default().fs(&fs))
            .expect(input)
    );
}
#[test]
fn imports_self() {
    let mut fs = TestFs::new();

    fs.add_file("input.scss", r#"@import "input";"#);

    let input = r#"
        @import "input";
    "#;

    assert_err!(
        input,
        "Error: This file is already being loaded.",
        &fugo_sass::Options::default().fs(&fs)
    );
}

#[test]
fn imports_explicit_file_extension() {
    let mut fs = TestFs::new();

    fs.add_file("a.scss", r#"a { color: red; }"#);

    let input = r#"
        @import "a.scss";
    "#;

    assert_eq!(
        "a {\n  color: red;\n}\n",
        &fugo_sass::from_string(input.to_string(), &fugo_sass::Options::default().fs(&fs))
            .expect(input)
    );
}

#[test]
fn potentially_conflicting_directory_and_file() {
    tempfile!(
        "index.scss",
        "$a: wrong;",
        dir = "potentially_conflicting_directory_and_file"
    );
    tempfile!(
        "_potentially_conflicting_directory_and_file.scss",
        "$a: right;"
    );

    let input = r#"
        @import "potentially_conflicting_directory_and_file";
        a {
            color: $a;
        }
    "#;

    assert_eq!(
        "a {\n  color: right;\n}\n",
        &fugo_sass::from_string(input.to_string(), &fugo_sass::Options::default()).expect(input)
    );
}

#[test]
fn finds_index_file_no_underscore() {
    tempfile!(
        "index.scss",
        "$a: right;",
        dir = "finds_index_file_no_underscore"
    );

    let input = r#"
        @import "finds_index_file_no_underscore";
        a {
            color: $a;
        }
    "#;

    assert_eq!(
        "a {\n  color: right;\n}\n",
        &fugo_sass::from_string(input.to_string(), &fugo_sass::Options::default()).expect(input)
    );
}
