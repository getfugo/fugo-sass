use super::*;

#[test]
fn use_can_see_modules_imported_by_other_modules_when_aliased_as_star() {
    let input = r#"
       @use "use_can_see_modules_imported_by_other_modules_when_aliased_as_star__a" as *;
       a { color: math.$e; }
    "#;

    tempfile!(
        "use_can_see_modules_imported_by_other_modules_when_aliased_as_star__a.scss",
        "@use \"sass:math\";"
    );

    assert_err!(
        r#"Error: There is no module with the namespace "math"."#,
        input
    );
}

#[test]
fn use_modules_imported_by_other_modules_does_not_cause_conflict() {
    let input = r#"
       @use "use_modules_imported_by_other_modules_does_not_cause_conflict__a" as a;
       @use "use_modules_imported_by_other_modules_does_not_cause_conflict__b" as b;"#;

    tempfile!(
        "use_modules_imported_by_other_modules_does_not_cause_conflict__a.scss",
        "$a: red;"
    );
    tempfile!(
        "use_modules_imported_by_other_modules_does_not_cause_conflict__b.scss",
        "@use \"use_modules_imported_by_other_modules_does_not_cause_conflict__a\" as a; a { color: a.$a; }"
    );

    assert_eq!(
        "a {\n  color: red;\n}\n",
        &fugo_sass::from_string(input.to_string(), &fugo_sass::Options::default()).expect(input)
    );
}

#[test]
fn use_mixin_can_use_scope_from_own_module() {
    let input = r#"
        @use "use_mixin_can_use_scope_from_own_module__a" as a;
        @include a.foo();
    "#;

    tempfile!(
        "use_mixin_can_use_scope_from_own_module__a.scss",
        "$a: red;

        @mixin foo() {
          a {
            color: $a;
          }
        }"
    );

    assert_eq!(
        "a {\n  color: red;\n}\n",
        &fugo_sass::from_string(input.to_string(), &fugo_sass::Options::default()).expect(input)
    );
}

#[test]
fn use_function_can_use_scope_from_own_module() {
    let input = r#"
        @use "use_function_can_use_scope_from_own_module__a" as a;

        a {
            color: a.foo();
        }
    "#;

    tempfile!(
        "use_function_can_use_scope_from_own_module__a.scss",
        "$a: red;

        @function foo() {
            @return $a;
        }"
    );

    assert_eq!(
        "a {\n  color: red;\n}\n",
        &fugo_sass::from_string(input.to_string(), &fugo_sass::Options::default()).expect(input)
    );
}

#[test]
fn use_variable_redeclaration_builtin() {
    let input = "@use \"sass:math\";\nmath.$e: red;";

    assert_err!("Error: Cannot modify built-in variable.", input);
}

#[test]
fn use_variable_declaration_between_use() {
    let input = r#"
        $a: red;
        $b: green;
        @use "sass:math";
        $b: red;
        @use "sass:meta";
        a {
            color: $a $b;
        }"#;

    assert_eq!(
        "a {\n  color: red red;\n}\n",
        &fugo_sass::from_string(input.to_string(), &fugo_sass::Options::default()).expect(input)
    );
}

#[test]
fn include_mixin_with_star_namespace() {
    let mut fs = TestFs::new();

    fs.add_file(
        "a.scss",
        r#"@mixin foo() {
            a {
                color: red;
            }
        }"#,
    );

    let input = r#"
        @use "a" as *;

        @include foo();
    "#;

    assert_eq!(
        "a {\n  color: red;\n}\n",
        &fugo_sass::from_string(input.to_string(), &fugo_sass::Options::default().fs(&fs))
            .expect(input)
    );
}

#[test]
fn include_variable_with_star_namespace() {
    let mut fs = TestFs::new();

    fs.add_file("a.scss", r#"$a: red;"#);

    let input = r#"
        @use "a" as *;

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
fn include_function_with_star_namespace() {
    let mut fs = TestFs::new();

    fs.add_file(
        "a.scss",
        r#"@function foo() {
            @return red;
        }"#,
    );

    let input = r#"
        @use "a" as *;

        a {
            color: foo();
        }
    "#;

    assert_eq!(
        "a {\n  color: red;\n}\n",
        &fugo_sass::from_string(input.to_string(), &fugo_sass::Options::default().fs(&fs))
            .expect(input)
    );
}

#[test]
fn use_with_through_forward_multiple() {
    let mut fs = TestFs::new();

    fs.add_file(
        "_used.scss",
        r#"
            @forward "left" with ($a: from used !default);
            @forward "right" with ($b: from used !default);
        "#,
    );
    fs.add_file(
        "_left.scss",
        r#"
            $a: from left !default;

            in-left {
                c: $a
            }
        "#,
    );
    fs.add_file(
        "_right.scss",
        r#"
            $b: from left !default;

            in-right {
                d: $b
            }
        "#,
    );

    let input = r#"
        @use "used" with ($a: from input, $b: from input);
    "#;

    assert_eq!(
        "in-left {\n  c: from input;\n}\n\nin-right {\n  d: from input;\n}\n",
        &fugo_sass::from_string(input.to_string(), &fugo_sass::Options::default().fs(&fs))
            .expect(input)
    );
}

#[test]
fn module_functions_empty() {
    let mut fs = TestFs::new();

    fs.add_file("_other.scss", r#""#);

    let input = r#"
        @use "sass:meta";
        @use "other";

        a {
            b: meta.inspect(meta.module-functions("other"))
        }
    "#;

    assert_eq!(
        "a {\n  b: ();\n}\n",
        &fugo_sass::from_string(input.to_string(), &fugo_sass::Options::default().fs(&fs))
            .expect(input)
    );
}

#[test]
fn module_functions_through_forward() {
    let mut fs = TestFs::new();

    fs.add_file(
        "_a.scss",
        r#"
        @forward "b";
    "#,
    );
    fs.add_file(
        "_b.scss",
        r#"
        @function foo() {}
    "#,
    );

    let input = r#"
        @use "sass:meta";
        @use "a";

        a {
            b: meta.inspect(meta.module-functions("a"))
        }
    "#;

    assert_eq!(
        "a {\n  b: (\"foo\": get-function(\"foo\"));\n}\n",
        &fugo_sass::from_string(input.to_string(), &fugo_sass::Options::default().fs(&fs))
            .expect(input)
    );
}

#[test]
fn use_variable_declared_in_this_and_other_module() {
    let mut fs = TestFs::new();

    fs.add_file(
        "_a.scss",
        r#"
        $a: blue;
    "#,
    );

    let input = r#"
        $a: red;
        @use "a" as *;

        a {
            color: $a;
        }
    "#;

    assert_err!(
        input,
        "Error: This module and the new module both define a variable named \"$a\".",
        fugo_sass::Options::default().fs(&fs)
    );
}

#[test]
#[ignore = "we don't check for this"]
fn use_variable_declared_in_two_modules() {
    let mut fs = TestFs::new();

    fs.add_file(
        "_a.scss",
        r#"
        $a: blue;
    "#,
    );

    fs.add_file(
        "_b.scss",
        r#"
        $a: red;
    "#,
    );

    let input = r#"
        @use "a" as *;
        @use "b" as *;

        a {
            color: $a;
        }
    "#;

    assert_err!(
        input,
        "Error: This variable is available from multiple global modules.",
        fugo_sass::Options::default().fs(&fs)
    );
}

#[test]
fn import_module_using_same_builtin_module() {
    let mut fs = TestFs::new();

    fs.add_file(
        "_a.scss",
        r#"
        @use "sass:meta";
    "#,
    );

    fs.add_file(
        "_b.scss",
        r#"
        $a: red;
    "#,
    );

    let input = r#"
        @use "sass:meta";
        @import "a";
    "#;

    assert_eq!(
        "",
        &fugo_sass::from_string(input.to_string(), &fugo_sass::Options::default().fs(&fs))
            .expect(input)
    );
}

#[test]
fn import_module_using_same_builtin_module_has_styles() {
    let mut fs = TestFs::new();

    fs.add_file(
        "_a.scss",
        r#"
        @use "sass:meta";

        a {
            color: red;
        }
    "#,
    );

    fs.add_file(
        "_b.scss",
        r#"
        $a: red;
    "#,
    );

    let input = r#"
        @use "sass:meta";
        @import "a";
    "#;

    assert_eq!(
        "a {\n  color: red;\n}\n",
        &fugo_sass::from_string(input.to_string(), &fugo_sass::Options::default().fs(&fs))
            .expect(input)
    );
}

#[test]
fn use_member_global_variable_assignment_toplevel() {
    let mut fs = TestFs::new();

    fs.add_file(
        "other.scss",
        r#"
            $member: value;

            @function get-member() {
                @return $member
            }
    "#,
    );

    let input = r#"
        @use "other" as *;

        $member: new value;
        
        a {
            b: get-member()
        }
    "#;

    assert_eq!(
        "a {\n  b: new value;\n}\n",
        &fugo_sass::from_string(input.to_string(), &fugo_sass::Options::default().fs(&fs))
            .expect(input)
    );
}
