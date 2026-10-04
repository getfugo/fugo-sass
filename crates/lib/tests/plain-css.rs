use fugo_sass::InputSyntax;

#[macro_use]
mod macros;

test!(
    function_call,
    "a {
        color: rotate(-45deg);
    }",
    "a {\n  color: rotate(-45deg);\n}\n",
    fugo_sass::Options::default().input_syntax(InputSyntax::Css)
);
test!(
    retains_null,
    "a {
        color: null;
    }",
    "a {\n  color: null;\n}\n",
    fugo_sass::Options::default().input_syntax(InputSyntax::Css)
);
test!(
    does_not_evaluate_and,
    "a {
        color: 1 and 2;
    }",
    "a {\n  color: 1 and 2;\n}\n",
    fugo_sass::Options::default().input_syntax(InputSyntax::Css)
);
test!(
    does_not_evaluate_or,
    "a {
        color: 1 or 2;
    }",
    "a {\n  color: 1 or 2;\n}\n",
    fugo_sass::Options::default().input_syntax(InputSyntax::Css)
);
test!(
    simple_calculation,
    "a {
        color: calc(1 + 1);
    }",
    "a {\n  color: 2;\n}\n",
    fugo_sass::Options::default().input_syntax(InputSyntax::Css)
);
test!(
    simple_url_import,
    r#"@import url("foo");"#,
    "@import url(\"foo\");\n",
    fugo_sass::Options::default().input_syntax(InputSyntax::Css)
);
test!(
    import_no_file_extension,
    r#"@import "foo";"#,
    "@import \"foo\";\n",
    fugo_sass::Options::default().input_syntax(InputSyntax::Css)
);
test!(
    import_with_condition,
    r#"@import "foo" screen and (foo: bar);"#,
    "@import \"foo\" screen and (foo: bar);\n",
    fugo_sass::Options::default().input_syntax(InputSyntax::Css)
);
test!(
    does_not_evaluate_not,
    "a {
        color: not 2;
        color: not true;
        color: not false;
    }",
    "a {\n  color: not 2;\n  color: not true;\n  color: not false;\n}\n",
    fugo_sass::Options::default().input_syntax(InputSyntax::Css)
);
error!(
    denies_silent_comment,
    "// silent",
    "Error: Silent comments aren't allowed in plain CSS.",
    fugo_sass::Options::default().input_syntax(InputSyntax::Css)
);
error!(
    denies_function_rule,
    "@function foo() {
        @return 2;
    }",
    "Error: This at-rule isn't allowed in plain CSS.",
    fugo_sass::Options::default().input_syntax(InputSyntax::Css)
);
error!(
    denies_content_rule,
    "@content",
    "Error: This at-rule isn't allowed in plain CSS.",
    fugo_sass::Options::default().input_syntax(InputSyntax::Css)
);
test!(
    allows_media_rule,
    "@media (foo) {
        a {
            color: red;
        }
    }",
    "@media (foo) {\n  a {\n    color: red;\n  }\n}\n",
    fugo_sass::Options::default().input_syntax(InputSyntax::Css)
);
test!(
    allows_var_empty_second_arg,
    "a {
        color: var(1, );
    }",
    "a {\n  color: var(1, );\n}\n",
    fugo_sass::Options::default().input_syntax(InputSyntax::Css)
);
error!(
    disallows_empty_second_arg_in_non_var_function,
    "a {
        color: foo(1, );
    }",
    "Error: Expected expression.",
    fugo_sass::Options::default().input_syntax(InputSyntax::Css)
);
error!(
    // Plain CSS parses `if()` as a CSS `if()`, which needs a condition such as `css()`.
    disallows_if_function,
    "a {
        color: if(true, a, b);
    }",
    "Error: expected \"(\".",
    fugo_sass::Options::default().input_syntax(InputSyntax::Css)
);
error!(
    disallows_map_get_function,
    "a {
        color: map-get(true, a, b);
    }",
    "Error: This function isn't allowed in plain CSS.",
    fugo_sass::Options::default().input_syntax(InputSyntax::Css)
);
error!(
    disallows_plus_operator,
    "a {
        color: 1 + 2;
    }",
    "Error: Operators aren't allowed in plain CSS.",
    fugo_sass::Options::default().input_syntax(InputSyntax::Css)
);
error!(
    disallows_parens,
    "a {
        color: (a b);
    }",
    "Error: Parentheses aren't allowed in plain CSS.",
    fugo_sass::Options::default().input_syntax(InputSyntax::Css)
);
error!(
    disallows_variable_expr,
    "a {
        color: $a;
    }",
    "Error: Sass variables aren't allowed in plain CSS.",
    fugo_sass::Options::default().input_syntax(InputSyntax::Css)
);
error!(
    disallows_variable_decl,
    "$bar: red;",
    "Error: Sass variables aren't allowed in plain CSS.",
    fugo_sass::Options::default().input_syntax(InputSyntax::Css)
);
error!(
    disallows_parent_selector_expr,
    "a {
        color: &;
    }",
    "Error: The parent selector isn't allowed in plain CSS.",
    fugo_sass::Options::default().input_syntax(InputSyntax::Css)
);
error!(
    disallows_unary_plus,
    "a {
        color: +(1);
    }",
    "Error: Operators aren't allowed in plain CSS.",
    fugo_sass::Options::default().input_syntax(InputSyntax::Css)
);
error!(
    disallows_unary_minus,
    "a {
        color: -(1);
    }",
    "Error: Operators aren't allowed in plain CSS.",
    fugo_sass::Options::default().input_syntax(InputSyntax::Css)
);
error!(
    disallows_interpolation,
    "a {
        color: a#{b}c;
    }",
    "Error: Interpolation isn't allowed in plain CSS.",
    fugo_sass::Options::default().input_syntax(InputSyntax::Css)
);
error!(
    disallows_placeholder_selector,
    "%a {
        color: red;
    }",
    "Error: Placeholder selectors aren't allowed in plain CSS.",
    fugo_sass::Options::default().input_syntax(InputSyntax::Css)
);
test!(
    allows_rgb_function,
    "a {
        color: rgb(true, a, b);
    }",
    "a {\n  color: rgb(true, a, b);\n}\n",
    fugo_sass::Options::default().input_syntax(InputSyntax::Css)
);
test!(
    simple_supports,
    "@supports (foo) {
        a {
            color: red;
        }
    }",
    "@supports (foo) {\n  a {\n    color: red;\n  }\n}\n",
    fugo_sass::Options::default().input_syntax(InputSyntax::Css)
);
test!(
    custom_property,
    "a {
        --foo: /* */;
    }",
    "a {\n  --foo: /* */;\n}\n",
    fugo_sass::Options::default().input_syntax(InputSyntax::Css)
);
error!(
    single_nested_property,
    "a {
        b: {
            c: d;
        }
    }",
    "Error: Nested declarations aren't allowed in plain CSS.",
    fugo_sass::Options::default().input_syntax(InputSyntax::Css)
);
error!(
    single_nested_property_with_expression,
    "a {
        b: 2 {
            c: d;
        }
    }",
    "Error: Nested declarations aren't allowed in plain CSS.",
    fugo_sass::Options::default().input_syntax(InputSyntax::Css)
);
test!(
    // dart-sass 1.73: nesting in plain CSS is written as is.
    nesting_is_kept,
    "a {\n  b: c;\n  & d {e: f}\n  @media screen {g: h}\n}\n",
    "a {\n  b: c;\n  & d {\n    e: f;\n  }\n}\n@media screen {\n  a {\n    g: h;\n  }\n}\n",
    fugo_sass::Options::default().input_syntax(InputSyntax::Css)
);
