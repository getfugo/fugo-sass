use super::*;

error!(
    after_style,
    "a {}
    @use \"foo\";
    ",
    "Error: @use rules must be written before any other rules."
);
error!(
    interpolation_in_as_identifier,
    "@use \"sass:math\" as m#{a}th;", "Error: expected \";\"."
);
error!(
    use_as_quoted_string,
    "@use \"sass:math\" as \"math\";", "Error: Expected identifier."
);
error!(
    use_as_missing_s,
    "@use \"sass:math\" a math;", "Error: expected \";\"."
);
error!(
    unknown_module_get_variable,
    "a { color: foo.$bar; }", "Error: There is no module with the namespace \"foo\"."
);
error!(
    unknown_module_get_function,
    "a { color: foo.bar(); }", "Error: There is no module with the namespace \"foo\"."
);
error!(
    unknown_function,
    "@use \"sass:math\";\na { color: math.bar(); }", "Error: Undefined function."
);
error!(
    module_function_missing_open_parens,
    "@use \"sass:math\";\na { color: math.floor; }", "Error: expected \"(\"."
);
error!(
    module_not_quoted_string,
    "@use a", "Error: Expected string."
);
error!(
    use_file_name_is_invalid_identifier,
    r#"@use "a b";"#, r#"Error: The default namespace "a b" is not a valid Sass identifier."#
);
error!(
    use_empty_string,
    r#"@use "";"#, r#"Error: The default namespace "" is not a valid Sass identifier."#
);
error!(
    configure_builtin_module,
    r#"@use "sass:math" with ($e: 5);"#, r#"Error: Built-in modules can't be configured."#
);
test!(
    use_as,
    "@use \"sass:math\" as foo;
    a {
        color: foo.clamp(0, 1, 2);
    }",
    "a {\n  color: 1;\n}\n"
);
test!(
    use_as_uppercase,
    "@use \"sass:math\" AS foo;
    a {
        color: foo.clamp(0, 1, 2);
    }",
    "a {\n  color: 1;\n}\n"
);
test!(
    use_as_universal,
    "@use \"sass:math\" as *;
    a {
        color: cos(2);
    }",
    "a {\n  color: -0.4161468365;\n}\n"
);
test!(
    use_single_quotes,
    "@use 'sass:math';
    a {
        color: math.cos(2);
    }",
    "a {\n  color: -0.4161468365;\n}\n"
);

#[test]
fn use_user_defined_same_directory() {
    let input = "@use \"use_user_defined_same_directory\";\na {\n color: use_user_defined_same_directory.$a;\n}";
    tempfile!(
        "use_user_defined_same_directory.scss",
        "$a: red; a { color: $a; }"
    );
    assert_eq!(
        "a {\n  color: red;\n}\n\na {\n  color: red;\n}\n",
        &fugo_sass::from_string(input.to_string(), &fugo_sass::Options::default()).expect(input)
    );
}

#[test]
fn private_variable_begins_with_underscore() {
    let mut fs = TestFs::new();

    fs.add_file(
        "_a.scss",
        r#"
        $_foo: red;
        a { color: $_foo; }
    "#,
    );

    let input = r#"
        @use "a" as module;
        b {
            color: module.$_foo;
        }
    "#;

    assert_err!(
        input,
        "Error: Private members can't be accessed from outside their modules.",
        &fugo_sass::Options::default().fs(&fs)
    );
}

#[test]
fn private_variable_begins_with_hyphen() {
    let mut fs = TestFs::new();

    fs.add_file(
        "_a.scss",
        r#"
        $-foo: red;
        a { color: $-foo; }
    "#,
    );

    let input = r#"
        @use "a" as module;
        b {
            color: module.$-foo
        }
    "#;

    assert_err!(
        input,
        "Error: Private members can't be accessed from outside their modules.",
        &fugo_sass::Options::default().fs(&fs)
    );
}

#[test]
fn private_function() {
    let mut fs = TestFs::new();

    fs.add_file(
        "_a.scss",
        r#"
        @function _foo($a) { @return $a; }
        a { color: _foo(red); }
    "#,
    );

    let input = r#"
        @use "a" as module;
        b {
            color: module._foo(green)
        }
    "#;

    assert_err!(
        input,
        "Error: Private members can't be accessed from outside their modules.",
        &fugo_sass::Options::default().fs(&fs)
    );
}

#[test]
fn global_variable_exists_private() {
    let mut fs = TestFs::new();

    fs.add_file(
        "_a.scss",
        r#"
        $foo: red;
        $_foo: red;
    "#,
    );

    let input = r#"
        @use "a" as module;
        a {
            color: global-variable-exists($name: foo, $module: module);
            color: global-variable-exists($name: _foo, $module: module);
        }
    "#;

    assert_eq!(
        "a {\n  color: true;\n  color: false;\n}\n",
        &fugo_sass::from_string(input.to_string(), &fugo_sass::Options::default().fs(&fs))
            .expect(input)
    );
}

#[test]
fn use_user_defined_as() {
    let mut fs = TestFs::new();

    fs.add_file(
        "_a.scss",
        r#"
        $a: red; a { color: $a; }
    "#,
    );

    let input = r#"
        @use "a" as module;
        a {
            color: module.$a;
        }
    "#;

    assert_eq!(
        "a {\n  color: red;\n}\n\na {\n  color: red;\n}\n",
        &fugo_sass::from_string(input.to_string(), &fugo_sass::Options::default().fs(&fs))
            .expect(input)
    );
}

#[test]
fn use_user_defined_function() {
    let mut fs = TestFs::new();

    fs.add_file(
        "_a.scss",
        r#"
        @function foo($a) { @return $a; }
    "#,
    );

    let input = r#"
        @use "a" as module;
        a {
            color: module.foo(red);
        }
    "#;

    assert_eq!(
        "a {\n  color: red;\n}\n",
        &fugo_sass::from_string(input.to_string(), &fugo_sass::Options::default().fs(&fs))
            .expect(input)
    );
}

#[test]
fn use_idempotent_no_alias() {
    let mut fs = TestFs::new();

    fs.add_file("_a.scss", r#""#);

    let input = r#"
        @use "a";
        @use "a";
    "#;

    assert_err!(
        input,
        "Error: There's already a module with namespace \"a\".",
        fugo_sass::Options::default().fs(&fs)
    );
}

#[test]
fn use_idempotent_with_alias() {
    let mut fs = TestFs::new();

    fs.add_file("_a.scss", r#""#);
    fs.add_file("_b.scss", r#""#);

    let input = r#"
        @use "a" as foo;
        @use "b" as foo;
    "#;

    assert_err!(
        input,
        "Error: There's already a module with namespace \"foo\".",
        fugo_sass::Options::default().fs(&fs)
    );
}

#[test]
fn use_idempotent_builtin() {
    let input = "@use \"sass:math\";\n@use \"sass:math\";\n";

    assert_err!(
        "Error: There's already a module with namespace \"math\".",
        input
    );
}

#[test]
fn use_with_simple() {
    let input = "@use \"use_with_simple\" with ($a: red);\na {\n color: use_with_simple.$a;\n}";
    tempfile!("use_with_simple.scss", "$a: green !default;");
    assert_eq!(
        "a {\n  color: red;\n}\n",
        &fugo_sass::from_string(input.to_string(), &fugo_sass::Options::default()).expect(input)
    );
}

#[test]
fn use_as_with() {
    let input = "@use \"use_as_with\" as module with ($a: red);\na {\n color: module.$a;\n}";
    tempfile!("use_as_with.scss", "$a: green !default;");
    assert_eq!(
        "a {\n  color: red;\n}\n",
        &fugo_sass::from_string(input.to_string(), &fugo_sass::Options::default()).expect(input)
    );
}

#[test]
fn use_whitespace_and_comments() {
    let input = "@use  /**/  \"use_whitespace_and_comments\"  /**/  as  /**/  foo  /**/  with  /**/  (  /**/  $a  /**/  :  /**/  red  /**/  );";
    tempfile!(
        "use_whitespace_and_comments.scss",
        "$a: green !default; a { color: $a }"
    );
    assert_eq!(
        "a {\n  color: red;\n}\n",
        &fugo_sass::from_string(input.to_string(), &fugo_sass::Options::default()).expect(input)
    );
}

#[test]
fn use_loud_comment_after_close_paren_with() {
    let input = r#"@use "b" as foo with ($a : red)  /**/  ;"#;
    tempfile!(
        "use_loud_comment_after_close_paren_with.scss",
        "$a: green !default; a { color: $a }"
    );
    assert_err!(r#"Error: expected ";"."#, input);
}

#[test]
fn use_with_builtin_module() {
    let input = "@use \"sass:math\" with ($e: 2.7);";

    assert_err!("Error: Built-in modules can't be configured.", input);
}

#[test]
fn use_with_variable_never_used() {
    let input = "@use \"use_with_variable_never_used\" with ($a: red);";
    tempfile!("use_with_variable_never_used.scss", "");

    assert_err!(
        "Error: This variable was not declared with !default in the @used module.",
        input
    );
}

#[test]
fn use_with_same_variable_multiple_times() {
    let input = "@use \"use_with_same_variable_multiple_times\" as foo with ($a: b, $a: c);";
    tempfile!("use_with_same_variable_multiple_times.scss", "");

    assert_err!(
        "Error: The same variable may only be configured once.",
        input
    );
}

#[test]
fn use_variable_redeclaration_var_dne() {
    let input = "@use \"use_variable_redeclaration_var_dne\" as mod;\nmod.$a: red;";
    tempfile!("use_variable_redeclaration_var_dne.scss", "");

    assert_err!("Error: Undefined variable.", input);
}

#[test]
fn use_variable_redeclaration_global() {
    let input = "@use \"use_variable_redeclaration_global\" as mod;\nmod.$a: red !global;";
    tempfile!("use_variable_redeclaration_global.scss", "$a: green;");

    assert_err!(
        "Error: !global isn't allowed for variables in other modules.",
        input
    );
}

#[test]
fn use_variable_redeclaration_simple() {
    let input =
        "@use \"use_variable_redeclaration_simple\" as mod;\nmod.$a: red; a { color: mod.$a; }";
    tempfile!("use_variable_redeclaration_simple.scss", "$a: green;");

    assert_eq!(
        "a {\n  color: red;\n}\n",
        &fugo_sass::from_string(input.to_string(), &fugo_sass::Options::default()).expect(input)
    );
}

#[test]
fn use_variable_redeclaration_default() {
    let input = "@use \"use_variable_redeclaration_default\" as mod;\nmod.$a: 1 % red !default; a { color: mod.$a; }";
    tempfile!("use_variable_redeclaration_default.scss", "$a: green;");

    assert_eq!(
        "a {\n  color: green;\n}\n",
        &fugo_sass::from_string(input.to_string(), &fugo_sass::Options::default()).expect(input)
    );
}

#[test]
fn use_variable_redeclaration_private() {
    let input = "@use \"use_variable_redeclaration_private\" as mod;\nmod.$-a: red;";
    tempfile!("use_variable_redeclaration_private.scss", "$a: green;");

    assert_err!(
        "Error: Private members can't be accessed from outside their modules.",
        input
    );
}

#[test]
fn use_cannot_see_modules_imported_by_other_modules() {
    let input = r#"
       @use "use_cannot_see_modules_imported_by_other_modules__a" as a;
       @use "use_cannot_see_modules_imported_by_other_modules__b" as b;"#;

    tempfile!(
        "use_cannot_see_modules_imported_by_other_modules__a.scss",
        "$a: green;"
    );
    tempfile!(
        "use_cannot_see_modules_imported_by_other_modules__b.scss",
        "a { color: a.$a; }"
    );

    assert_err!("Error: There is no module with the namespace \"a\".", input);
}
