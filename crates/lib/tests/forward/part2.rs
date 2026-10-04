use super::*;

#[test]
fn forward_with_through_forward_show() {
    let mut fs = TestFs::new();

    fs.add_file(
        "_downstream.scss",
        r#"
        @forward "midstream" with ($a: configured);
    "#,
    );
    fs.add_file(
        "_midstream.scss",
        r#"
        @forward "upstream" show $a;
    "#,
    );
    fs.add_file(
        "_upstream.scss",
        r#"
        $a: original !default;
        b {c: $a}
    "#,
    );

    let input = r#"
        @use "downstream";
    "#;

    assert_eq!(
        "b {\n  c: configured;\n}\n",
        &fugo_sass::from_string(input.to_string(), &fugo_sass::Options::default().fs(&fs))
            .expect(input)
    );
}

#[test]
#[ignore = "incorrectly thinks there's a module loop"]
fn import_forwarded_first_no_use() {
    let mut fs = TestFs::new();

    fs.add_file(
        "first.scss",
        r#"
        $variable: value;
    "#,
    );
    fs.add_file(
        "first.import.scss",
        r#"
        @forward "first";
    "#,
    );
    fs.add_file(
        "second.scss",
        r#"
        a {
            b: $variable;
        }
    "#,
    );

    let input = r#"
        @import "first";
        @import "second";
    "#;

    assert_eq!(
        "a {\n  b: value;\n}\n",
        &fugo_sass::from_string(input.to_string(), &fugo_sass::Options::default().fs(&fs))
            .expect(input)
    );
}

#[test]
fn forward_same_module_with_and_without_prefix() {
    let mut fs = TestFs::new();

    fs.add_file(
        "_midstream.scss",
        r#"
            @forward "upstream";
            @forward "upstream" as b-*;
        "#,
    );
    fs.add_file(
        "_upstream.scss",
        r#"
            @mixin a() {
                c {
                    d: e
                }
            }
        "#,
    );

    let input = r#"
        @use "midstream";

        @include midstream.a;
    "#;

    assert_eq!(
        "c {\n  d: e;\n}\n",
        &fugo_sass::from_string(input.to_string(), &fugo_sass::Options::default().fs(&fs))
            .expect(input)
    );
}

error!(
    after_style_rule,
    r#"
        a {}
        @forward "foo";
    "#,
    "Error: @forward rules must be written before any other rules."
);

#[test]
fn hidden_mixin_is_not_forwarded() {
    let mut fs = TestFs::new();

    fs.add_file("_midstream.scss", r#"@forward "upstream" hide c;"#);
    fs.add_file("_upstream.scss", r#"@mixin c {a: b}"#);

    let input = "@use \"midstream\" as *;\na {\n  @include c;\n}\n";

    assert_eq!(
        "Error: Undefined mixin.",
        fugo_sass::from_string(input.to_string(), &fugo_sass::Options::default().fs(&fs))
            .unwrap_err()
            .to_string()
            .lines()
            .next()
            .unwrap()
    );
}

#[test]
fn prefixed_member_is_not_hidden_by_its_unprefixed_name() {
    let mut fs = TestFs::new();

    fs.add_file("_midstream.scss", r#"@forward "upstream" as b-* hide a;"#);
    fs.add_file("_upstream.scss", r#"@mixin a {c {d: e}}"#);

    let input = "@use \"midstream\";\n@include midstream.b-a;\n";

    assert_eq!(
        "c {\n  d: e;\n}\n",
        &fugo_sass::from_string(input.to_string(), &fugo_sass::Options::default().fs(&fs))
            .expect(input)
    );
}

#[test]
fn shown_function_is_the_only_one_forwarded() {
    let mut fs = TestFs::new();

    fs.add_file("_midstream.scss", r#"@forward "upstream" show c;"#);
    fs.add_file(
        "_upstream.scss",
        "@function c() {@return c}\n@function d() {@return d}\n",
    );

    let input = "@use \"sass:meta\";\n@use \"midstream\";\na {\n  b: meta.module-functions(midstream) == (\"c\": meta.get-function(c, $module: midstream));\n}\n";

    assert_eq!(
        "a {\n  b: true;\n}\n",
        &fugo_sass::from_string(input.to_string(), &fugo_sass::Options::default().fs(&fs))
            .expect(input)
    );
}
