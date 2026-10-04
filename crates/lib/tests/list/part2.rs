use super::*;

test!(
    list_separator_of_unquoted_string,
    "a {\n  color: list-separator(a);\n}\n",
    "a {\n  color: space;\n}\n"
);
test!(
    list_separator_of_arglist,
    "@function foo($a...) {
        @return list-separator($a);
    }
    a {
        color: foo();
    }",
    "a {\n  color: comma;\n}\n"
);
test!(
    list_separator_of_empty_list_after_join,
    "a {
        color: list-separator(join(join((), (), comma), 1 2));
        color: list-separator(join(join((), (), comma), (1, 2)));
    }",
    "a {\n  color: comma;\n  color: comma;\n}\n"
);
test!(
    slash_list_are_equal,
    "@use 'sass:list';
    a {
        color: list.slash(a, b)==list.slash(a, b);
    }",
    "a {\n  color: true;\n}\n"
);
test!(
    list_separator_slash,
    "@use 'sass:list';
    a {
        color: list-separator(list.slash(a, b));
    }",
    "a {\n  color: slash;\n}\n"
);
test!(
    list_slash,
    "@use 'sass:list';
    a {
        color: list.slash(a, b, c);
    }",
    "a {\n  color: a / b / c;\n}\n"
);
error!(
    nth_list_index_0,
    "a {\n  color: nth(a b c, 0);\n}\n", "Error: $n: List index may not be 0."
);
error!(
    invalid_item_in_space_separated_list,
    "a {\n  color: red color * #abc;\n}\n", "Error: Undefined operation \"color * #abc\"."
);
error!(
    invalid_item_in_comma_separated_list,
    "a {\n  color: red, color * #abc;\n}\n", "Error: Undefined operation \"color * #abc\"."
);
error!(
    invalid_item_in_space_separated_list_inside_interpolation,
    "a {\n  color: #{red color * #abc};\n}\n", "Error: Undefined operation \"color * #abc\"."
);
error!(
    invalid_item_in_comma_separated_list_inside_interpolation,
    "a {\n  color: #{red, color * #abc};\n}\n", "Error: Undefined operation \"color * #abc\"."
);
error!(
    nth_invalid_index_message_contains_unit,
    "a {\n  color: nth([], 1px);\n}\n", "Error: $n: Invalid index 1px for a list with 0 elements."
);
error!(
    set_nth_invalid_index_message_contains_unit,
    "a {\n  color: set-nth([], 1px, a);\n}\n",
    "Error: $n: Invalid index 1px for a list with 0 elements."
);
error!(
    #[ignore = "we don't error"]
    empty_list_is_invalid,
    "a {\n  color: ();\n}\n", "Error: () isn't a valid CSS value."
);
test!(
    is_bracketed_empty_bracket_list,
    "a {\n  color: is-bracketed([]);\n}\n",
    "a {\n  color: true;\n}\n"
);
test!(
    is_bracketed_bracket_list_containing_space_list,
    "a {\n  color: is-bracketed([a b]);\n}\n",
    "a {\n  color: true;\n}\n"
);
test!(
    is_bracketed_bracket_list_containing_comma_list,
    "a {\n  color: is-bracketed([a, b]);\n}\n",
    "a {\n  color: true;\n}\n"
);
test!(
    is_bracketed_space_list,
    "a {\n  color: is-bracketed(a b);\n}\n",
    "a {\n  color: false;\n}\n"
);
test!(
    is_bracketed_number,
    "a {\n  color: is-bracketed(1);\n}\n",
    "a {\n  color: false;\n}\n"
);
error!(
    is_bracketed_two_args,
    "a {\n  color: is-bracketed(a, b);\n}\n", "Error: Only 1 argument allowed, but 2 were passed."
);
error!(
    nth_non_numeric_index,
    "a {\n  color: nth(a b, c);\n}\n", "Error: $n: c is not a number."
);
error!(
    set_nth_non_numeric_index,
    "a {\n  color: set-nth(a b, c, d);\n}\n", "Error: $n: c is not a number."
);
error!(
    set_nth_index_zero,
    "a {\n  color: set-nth(a b, 0, d);\n}\n", "Error: $n: List index may not be 0."
);
error!(
    set_nth_index_decimal,
    "a {\n  color: set-nth(a b, 1.5, d);\n}\n", "Error: $n: 1.5 is not an int."
);
error!(
    set_nth_index_negative_outside_range,
    "a {\n  color: set-nth(a b, -3, d);\n}\n",
    "Error: $n: Invalid index -3 for a list with 2 elements."
);
test!(
    set_nth_index_negative_inside_range,
    "a {\n  color: set-nth(a b, -1, d);\n}\n",
    "a {\n  color: a d;\n}\n"
);
error!(
    set_nth_index_infinity,
    "a {\n  color: set-nth(a b, 1/0, d);\n}\n", "Error: $n: calc(infinity) is not an int."
);
error!(
    set_nth_index_negative_infinity,
    "a {\n  color: set-nth(a b, -1/0, d);\n}\n", "Error: $n: calc(-infinity) is not an int."
);
error!(
    set_nth_decimal_outside_range,
    "a {\n  color: set-nth(a b, 8.5, d);\n}\n", "Error: $n: 8.5 is not an int."
);
test!(
    append_with_slash_separator,
    "a {\n  color: append(a b, c, slash);\n}\n",
    "a {\n  color: a / b / c;\n}\n"
);
error!(
    append_invalid_separator,
    "a {\n  color: append(a b, c, foo);\n}\n",
    "Error: $separator: Must be \"space\", \"comma\", \"slash\", or \"auto\"."
);
test!(
    join_with_slash_separator,
    "a {\n  color: join(a, b, slash);\n}\n",
    "a {\n  color: a / b;\n}\n"
);
error!(
    join_invalid_separator,
    "a {\n  color: join(a b, c, foo);\n}\n",
    "Error: $separator: Must be \"space\", \"comma\", \"slash\", or \"auto\"."
);
error!(
    join_invalid_separator_non_string,
    "a {\n  color: join(a b, c, 1);\n}\n", "Error: $separator: 1 is not a string."
);
test!(
    join_bracketed_true,
    "a {\n  color: join(a, b, space, true);\n}\n",
    "a {\n  color: [a b];\n}\n"
);
test!(
    join_bracketed_truthy,
    "a {\n  color: join(a, b, space, a);\n}\n",
    "a {\n  color: [a b];\n}\n"
);
test!(
    join_bracketed_falsey,
    "a {\n  color: join(a, b, space, null);\n}\n",
    "a {\n  color: a b;\n}\n"
);
error!(
    zip_no_args,
    "a {\n  color: zip();\n}\n", "Error: () isn't a valid CSS value."
);
