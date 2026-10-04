use super::*;

test!(
    pseudo_element_single_quotes,
    r#"::foo('red') {
        color: &;
    }"#,
    "::foo('red') {\n  color: ::foo('red');\n}\n"
);
test!(
    pseudo_element_loud_comments,
    r#"::foo(/**/a/**/b/**/) {
        color: &;
    }"#,
    "::foo(a/**/b/**/) {\n  color: ::foo(a/**/b/**/);\n}\n"
);
test!(
    pseudo_element_forward_slash,
    r#"::foo(/a/b/) {
        color: &;
    }"#,
    "::foo(/a/b/) {\n  color: ::foo(/a/b/);\n}\n"
);
test!(
    interpolated_parent_selector_as_child_to_selector_with_escape_and_length_greater_than_child,
    r#"abcde \a {
        #{&} {
            color: red;
        }
    }"#,
    "abcde \\a  abcde \\a  {\n  color: red;\n}\n"
);
error!(
    interpolated_parent_selector_as_child_to_selector_with_escape_and_invalid_escape_and_length_greater_than_child,
    r#"abcde \a {
        #{&} \1111111 {
            color: red;
        }
    }"#,
    "Error: Invalid Unicode code point."
);
test!(
    interpolated_parent_selector_as_child_to_selector_with_attribute_selector_and_length_greater_than_child,
    r#"abcde [a] {
        #{&} {
            color: red;
        }
    }"#,
    "abcde [a] abcde [a] {\n  color: red;\n}\n"
);
error!(
    pseudo_element_interpolated_semicolon_no_brackets,
    r#"::foo(#{";"}) {
        color: &;
    }"#,
    r#"Error: expected ")"."#
);
test!(
    pseudo_element_interpolated_semicolon_with_parens,
    r#"::foo((#{";"})) {
        color: &;
    }"#,
    "::foo((;)) {\n  color: ::foo((;));\n}\n"
);
error!(
    a_n_plus_b_n_invalid_odd,
    ":nth-child(ofdd) {\n  color: &;\n}\n", "Error: Expected \"odd\"."
);
error!(
    a_n_plus_b_n_invalid_starting_char,
    ":nth-child(f) {\n  color: &;\n}\n", "Error: Expected \"n\"."
);
error!(
    a_n_plus_b_n_nothing_after_open_paren,
    ":nth-child({\n  color: &;\n}\n", "Error: expected more input."
);
error!(
    a_n_plus_b_n_invalid_char_after_even,
    ":nth-child(even#) {\n  color: &;\n}\n", "Error: expected \")\"."
);
error!(
    a_n_plus_b_n_double_nothing_after_plus,
    ":nth-child:nth-child(n+{}", "Error: Expected a number."
);
error!(
    a_n_plus_b_n_nothing_after_plus,
    ":nth-child(n+{}", "Error: Expected a number."
);
error!(
    a_n_plus_b_n_non_numeric_after_plus,
    ":nth-child(n+b) {}", "Error: Expected a number."
);
error!(nothing_after_period, ". {}", "Error: Expected identifier.");
error!(nothing_after_hash, "# {}", "Error: Expected identifier.");
error!(nothing_after_percent, "% {}", "Error: Expected identifier.");
error!(no_ident_after_colon, ": {}", "Error: Expected identifier.");
error!(double_colon_no_space, "::{}", "Error: Expected identifier.");
error!(
    non_ident_char_after_colon,
    ":#ab {}", "Error: Expected identifier."
);
error!(nothing_after_colon, "a:{}", "Error: Expected identifier.");
test!(toplevel_parent_selector_after_combinator, "~&{}", "");
error!(
    toplevel_parent_selector_after_element,
    "a&{}", "Error: \"&\" may only used at the beginning of a compound selector."
);
error!(
    denies_optional_in_selector,
    "a !optional {}", "Error: expected \"{\"."
);
error!(
    child_selector_starts_with_forward_slash,
    "a { /b { } }", "Error: expected selector."
);
test!(
    selector_module_exists,
    "@use 'sass:selector';
    a {
        color: selector.parse('a');
    }
    ",
    "a {\n  color: a;\n}\n"
);
test!(
    selector_contains_url_without_parens,
    "a url {\n  color: red;\n}\n",
    "a url {\n  color: red;\n}\n"
);
test!(
    selector_contains_capital_u,
    "a U {\n  color: red;\n}\n",
    "a U {\n  color: red;\n}\n"
);
test!(
    selector_contains_url_with_quoted_string_inside_parens,
    "a :url(\"foo.css\") {\n  color: red;\n}\n",
    "a :url(\"foo.css\") {\n  color: red;\n}\n"
);
test!(
    selector_contains_url_with_hash_inside_parens,
    "a :url(#) {\n  color: red;\n}\n",
    "a :url(#) {\n  color: red;\n}\n"
);
test!(
    attr_val_is_url,
    "[attr=url] {\n  color: &;\n}\n",
    "[attr=url] {\n  color: [attr=url];\n}\n"
);
test!(
    attr_val_starts_with_u,
    "[attr=unit] {\n  color: &;\n}\n",
    "[attr=unit] {\n  color: [attr=unit];\n}\n"
);
error!(
    nth_child_loud_comment_between_n_and_of,
    ":nth-child(n/**/of a) {\n  color: &;\n}\n", "Error: expected \")\"."
);
test!(
    // dart-sass 1.99: a parent selector at the root of the document is written as is.
    toplevel_parent_selector_is_kept,
    "& {a: b}\n& .c {d: e}\n",
    "& {\n  a: b;\n}\n\n& .c {\n  d: e;\n}\n"
);
error!(
    toplevel_parent_selector_with_suffix,
    "&-a {b: c}", "Error: A top-level selector may not contain a parent selector with a suffix."
);
