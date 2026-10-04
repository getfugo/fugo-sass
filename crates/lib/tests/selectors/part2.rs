use super::*;

error!(
    allows_id_start_with_number,
    "#2foo {\n  color: red;\n}\n", "Error: Expected identifier."
);
error!(
    allows_id_only_number,
    "#2 {\n  color: red;\n}\n", "Error: Expected identifier."
);
test!(
    id_interpolation,
    "$zzz: zzz;\n##{$zzz} {\n  a: b;\n}\n",
    "#zzz {\n  a: b;\n}\n"
);
test!(
    interpolate_id_selector,
    "$bar: \"#foo\";\nul li#{$bar} {\n  foo: bar;\n}\n",
    "ul li#foo {\n  foo: bar;\n}\n"
);
test!(
    escaped_space,
    "a\\ b {\n  color: foo;\n}\n",
    "a\\ b {\n  color: foo;\n}\n"
);
test!(
    escaped_bang,
    "\\! {\n  color: red;\n}\n",
    "\\! {\n  color: red;\n}\n"
);
test!(
    multiple_consecutive_immediate_child,
    "> > foo {\n  color: foo;\n}\n",
    "> > foo {\n  color: foo;\n}\n"
);
error!(
    modifier_on_any_attr,
    "[attr i] {color: foo;}", "Error: Expected \"]\"."
);
test!(
    psuedo_paren_child_contains_ampersand,
    ".a, .b {\n  :not(&-c) {\n    d: e\n  }\n}\n",
    ":not(.a-c, .b-c) {\n  d: e;\n}\n"
);
test!(
    psuedo_paren_child_no_ampersand_two_newlines__this_test_confounds_me,
    ".a, .b {\n  :not(-c),\n  :not(-c) {\n    d: e\n  }\n}\n",
    ".a :not(-c),\n.a :not(-c), .b :not(-c),\n.b :not(-c) {\n  d: e;\n}\n"
);
test!(
    psuedo_paren_child_ampersand_two_newlines__this_test_confounds_me,
    ".a, .b {\n  :not(&-c, &-d),\n  :not(&-c) {\n    d: e\n  }\n}\n",
    ":not(.a-c, .a-d, .b-c, .b-d), :not(.a-c, .b-c) {\n  d: e;\n}\n"
);
test!(
    psuedo_paren_child_ampersand_inner_psuedo_paren,
    ".a, .b {\n  :not(:not(&-c)) {\n    d: e\n  }\n}\n",
    ":not(:not(.a-c, .b-c)) {\n  d: e;\n}\n"
);
test!(
    psuedo_paren_child_psuedo_paren_ampersand_inner_psuedo_paren,
    ".a, .b {\n  :not(:not(c), &-d) {\n    d: e\n  }\n}\n",
    ":not(:not(c), .a-d, .b-d) {\n  d: e;\n}\n"
);
test!(
    psuedo_paren_child_ampersand_psuedo_paren__inner_psuedo_paren,
    ".a, .b {\n  :not(&-d, :not(c)) {\n    d: e\n  }\n}\n",
    ":not(.a-d, :not(c), .b-d) {\n  d: e;\n}\n"
);
test!(
    sass_spec__nesting_parent_with_newline,
    ".foo,\n.bar {\n  .baz & {\n    color: red;\n  }\n}\n",
    ".baz .foo,\n.baz .bar {\n  color: red;\n}\n"
);
test!(
    not_only_placeholder_is_universal,
    "a :not(%b) {x: y}",
    "a * {\n  x: y;\n}\n"
);
test!(
    not_placeholder_is_removed,
    "a:not(%b, c) {x: y}",
    "a:not(c) {\n  x: y;\n}\n"
);
test!(
    psuedo_paren_removes_inner_placeholder_matches,
    "a:matches(%b, c) {x: y}",
    "a:matches(c) {\n  x: y;\n}\n"
);
test!(
    matches_placeholder_removes_everything_matches,
    "a:matches(%b) {x: y}",
    ""
);
test!(
    psuedo_paren_removes_inner_placeholder_is,
    "a:is(%b, c) {x: y}",
    "a:is(c) {\n  x: y;\n}\n"
);
test!(is_placeholder_removes_everything_is, "a:is(%b) {x: y}", "");
test!(
    touching_universal_stays_the_same,
    "a* {\n  color: red;\n}\n",
    "a* {\n  color: red;\n}\n"
);
test!(
    adjacent_not_placeholder_is_ignored,
    "a:not(%b) {x: y}",
    "a {\n  x: y;\n}\n"
);
test!(
    pseudo_paren_placeholder_alone,
    ":not(%b) {x: y}",
    "* {\n  x: y;\n}\n"
);
test!(
    interpolated_super_selector_in_style,
    "a {\n  color: #{&};\n}\n",
    "a {\n  color: a;\n}\n"
);
test!(
    interpolated_super_selector_in_style_symbols,
    "* .a #b:foo() {\n  color: #{&};\n}\n",
    "* .a #b:foo() {\n  color: * .a #b:foo();\n}\n"
);
test!(
    uninterpolated_super_selector,
    "* .a #b:foo() {\n  color: &;\n}\n",
    "* .a #b:foo() {\n  color: * .a #b:foo();\n}\n"
);
test!(
    interpolated_super_selector_in_selector_and_style,
    "a {\n  b #{&} {\n    color: &;\n  }\n}\n",
    "a b a {\n  color: a b a;\n}\n"
);
test!(
    super_selector_treated_as_list,
    "a, b {\n  color: type-of(&);\n}\n",
    "a, b {\n  color: list;\n}\n"
);
test!(
    length_of_comma_separated_super_selector,
    "a, b {\n  color: length(&);\n}\n",
    "a, b {\n  color: 2;\n}\n"
);
test!(
    nth_1_of_comma_separated_super_selector,
    "a, b {\n  color: nth(&, 1);\n}\n",
    "a, b {\n  color: a;\n}\n"
);
test!(
    nth_2_of_comma_separated_super_selector,
    "a, b {\n  color: nth(&, 2);\n}\n",
    "a, b {\n  color: b;\n}\n"
);
test!(
    length_of_space_separated_super_selector,
    "a b {\n  color: length(&);\n}\n",
    "a b {\n  color: 1;\n}\n"
);
test!(
    nth_1_of_space_separated_super_selector,
    "a b {\n  color: nth(&, 1);\n}\n",
    "a b {\n  color: a b;\n}\n"
);
test!(
    length_of_comma_separated_super_selector_has_compound,
    "a:foo, b {\n  color: length(&);\n}\n",
    "a:foo, b {\n  color: 2;\n}\n"
);
test!(
    nth_1_of_comma_separated_super_selector_has_compound,
    "a:foo, b {\n  color: nth(&, 1);\n}\n",
    "a:foo, b {\n  color: a:foo;\n}\n"
);
test!(
    length_of_space_separated_super_selector_has_compound,
    "a:foo b {\n  color: length(&);\n}\n",
    "a:foo b {\n  color: 1;\n}\n"
);
test!(
    nth_1_of_space_separated_super_selector_has_compound,
    "a:foo b {\n  color: nth(&, 1);\n}\n",
    "a:foo b {\n  color: a:foo b;\n}\n"
);
test!(
    length_super_selector_placeholder,
    "a, %b {\n  color: length(&);\n}\n",
    "a {\n  color: 2;\n}\n"
);
test!(
    nth_2_super_selector_placeholder,
    "a, %b {\n  color: nth(&, 2);\n}\n",
    "a {\n  color: %b;\n}\n"
);
test!(
    escape_at_start_of_selector,
    "\\61 {\n    color: red;\n}\n",
    "a {\n  color: red;\n}\n"
);
// this checks for a bug in which the selector
// contents get longer as a result of interpolation,
// which interferes with span information.
test!(
    selector_span_gets_larger,
    "$a: aaaaaaaaaaa;\n\n#{$a} {\n    color: foo;\n}\n",
    "aaaaaaaaaaa {\n  color: foo;\n}\n"
);
test!(
    toplevel_non_ascii_alphabetic,
    "ℓ {\n  color: red;\n}\n",
    "@charset \"UTF-8\";\nℓ {\n  color: red;\n}\n"
);
test!(
    plus_in_selector,
    "+ {\n  color: &;\n}\n",
    "+ {\n  color: +;\n}\n"
);
test!(
    invalid_chars_in_pseudo_parens,
    ":c(@#$) {\n  color: &;\n}\n",
    ":c(@#$) {\n  color: :c(@#$);\n}\n"
);
test!(
    empty_namespace_element,
    "|f {\n  color: &;\n}\n",
    "|f {\n  color: |f;\n}\n"
);
test!(
    universal_with_namespace,
    "a|* {\n  color: &;\n}\n",
    "a|* {\n  color: a|*;\n}\n"
);
test!(
    psuedo_element_slotted_args,
    "::slotted(b, c) {\n  color: &;\n}\n",
    "::slotted(b, c) {\n  color: ::slotted(b, c);\n}\n"
);
test!(
    a_n_plus_b,
    ":nth-child(2n+0) {\n  color: &;\n}\n",
    ":nth-child(2n+0) {\n  color: :nth-child(2n+0);\n}\n"
);
test!(
    a_n_plus_b_leading_negative,
    ":nth-child(-1n+6) {\n  color: &;\n}\n",
    ":nth-child(-1n+6) {\n  color: :nth-child(-1n+6);\n}\n"
);
test!(
    a_n_plus_b_leading_plus,
    ":nth-child(+3n-2) {\n  color: &;\n}\n",
    ":nth-child(+3n-2) {\n  color: :nth-child(+3n-2);\n}\n"
);
test!(
    a_n_plus_b_n_alone,
    ":nth-child(n) {\n  color: &;\n}\n",
    ":nth-child(n) {\n  color: :nth-child(n);\n}\n"
);
test!(
    a_n_plus_b_capital_n,
    ":nth-child(N) {\n  color: &;\n}\n",
    ":nth-child(n) {\n  color: :nth-child(n);\n}\n"
);
test!(
    a_n_plus_b_n_with_leading_number,
    ":nth-child(2n) {\n  color: &;\n}\n",
    ":nth-child(2n) {\n  color: :nth-child(2n);\n}\n"
);
test!(
    a_n_plus_b_n_whitespace_on_both_sides,
    ":nth-child(3n + 1) {\n  color: &;\n}\n",
    ":nth-child(3n+1) {\n  color: :nth-child(3n+1);\n}\n"
);
test!(
    a_n_plus_b_n_of,
    ":nth-child(2n+1 of b, c) {\n  color: &;\n}\n",
    ":nth-child(2n+1 of b, c) {\n  color: :nth-child(2n+1 of b, c);\n}\n"
);
test!(
    a_n_plus_b_n_number_alone,
    ":nth-child(5) {\n  color: &;\n}\n",
    ":nth-child(5) {\n  color: :nth-child(5);\n}\n"
);
test!(
    a_n_plus_b_n_number_leading_negative,
    ":nth-child(-5) {\n  color: &;\n}\n",
    ":nth-child(-5) {\n  color: :nth-child(-5);\n}\n"
);
test!(
    a_n_plus_b_n_number_leading_plus,
    ":nth-child(+5) {\n  color: &;\n}\n",
    ":nth-child(+5) {\n  color: :nth-child(+5);\n}\n"
);
test!(
    a_n_plus_b_n_leading_negative_no_leading_number,
    ":nth-child(-n+ 6) {\n  color: &;\n}\n",
    ":nth-child(-n+6) {\n  color: :nth-child(-n+6);\n}\n"
);
test!(
    a_n_plus_b_n_even_all_lowercase,
    ":nth-child(even) {\n  color: &;\n}\n",
    ":nth-child(even) {\n  color: :nth-child(even);\n}\n"
);
test!(
    a_n_plus_b_n_even_mixed_case,
    ":nth-child(eVeN) {\n  color: &;\n}\n",
    ":nth-child(even) {\n  color: :nth-child(even);\n}\n"
);
test!(
    a_n_plus_b_n_even_uppercase,
    ":nth-child(EVEN) {\n  color: &;\n}\n",
    ":nth-child(even) {\n  color: :nth-child(even);\n}\n"
);
test!(
    a_n_plus_b_n_even_whitespace,
    ":nth-child(    even   ) {\n  color: &;\n}\n",
    ":nth-child(even) {\n  color: :nth-child(even);\n}\n"
);
error!(
    a_n_plus_b_n_value_after_even,
    ":nth-child(even 1) {\n  color: &;\n}\n", "Error: Expected \"of\"."
);
error!(
    a_n_plus_b_n_invalid_even,
    ":nth-child(efven) {\n  color: &;\n}\n", "Error: Expected \"even\"."
);
test!(
    a_n_plus_b_n_odd_all_lowercase,
    ":nth-child(odd) {\n  color: &;\n}\n",
    ":nth-child(odd) {\n  color: :nth-child(odd);\n}\n"
);
test!(
    a_n_plus_b_n_odd_mixed_case,
    ":nth-child(oDd) {\n  color: &;\n}\n",
    ":nth-child(odd) {\n  color: :nth-child(odd);\n}\n"
);
test!(
    a_n_plus_b_n_odd_uppercase,
    ":nth-child(ODD) {\n  color: &;\n}\n",
    ":nth-child(odd) {\n  color: :nth-child(odd);\n}\n"
);
test!(
    a_n_plus_b_n_alone_of,
    ":nth-child(n of a) {\n  color: &;\n}\n",
    ":nth-child(n of a) {\n  color: :nth-child(n of a);\n}\n"
);
test!(
    escaped_space_at_end_of_selector_immediately_after_pseudo_color,
    "a color:\\  {\n  color: &;\n}\n",
    "a color:\\  {\n  color: a color:\\ ;\n}\n"
);
test!(
    super_selector_is_null_when_at_root,
    "@mixin foo {\n    #{if(&, 'true', 'false')} {\n        color: red;\n    }\n}\n\n@include foo;\n\na {\n    @include foo;\n}\n",
    "false {\n  color: red;\n}\n\na true {\n  color: red;\n}\n"
);
test!(
    newline_is_preserved_when_following_comment,
    "a, // 1\nb,\nc {\n    color: red;\n}\n",
    "a,\nb,\nc {\n  color: red;\n}\n"
);
test!(
    spaces_are_preserved_before_comma_in_pseudo_arg,
    ":a(a , b) {\n  color: &;\n}\n",
    ":a(a , b) {\n  color: :a(a , b);\n}\n"
);
test!(
    parent_selector_is_null_at_root,
    "#{inspect(&)}  {\n  color: &;\n}\n",
    "null {\n  color: null;\n}\n"
);
error!(
    id_selector_starts_with_number,
    "#2b  {\n  color: &;\n}\n", "Error: Expected identifier."
);
test!(
    nth_of_type_mutliple_spaces_inside_parens_are_collapsed,
    ":nth-of-type(2  n  -  --1) {\n  color: red;\n}\n",
    ":nth-of-type(2 n - --1) {\n  color: red;\n}\n"
);
test!(
    silent_comment_in_quoted_attribute_value,
    ".foo bar[val=\"//\"] {\n  color: &;\n}\n",
    ".foo bar[val=\"//\"] {\n  color: .foo bar[val=\"//\"];\n}\n"
);
test!(
    attribute_value_escape_ends_before_non_hex_char,
    "[a=\"a\\\\66\"] {\n  color: &;\n}\n",
    "[a=a\\66] {\n  color: [a=a\\66];\n}\n"
);
test!(
    attribute_value_contains_escaped_slash_in_quotes,
    r#"[data-key="\\"] {
        color: &;
    }"#,
    "[data-key=\"\\\\\"] {\n  color: [data-key=\"\\\\\"];\n}\n"
);
test!(
    #[ignore = "we have to rewrite quoted attribute serialization"]
    attribute_value_escape_ends_with_whitespace,
    r#"[a="a\\66  "] {  color: &;}"#,
    "[a=\"a\\\\66  \"] {\n  color: [a=\"a\\\\66  \"];\n}\n"
);

test!(
    no_newline_between_styles_when_last_style_was_placeholder,
    "a {
      color: red;
    
      %b {
        color: red;
      }
    }
    
    c {
      color: red;
    }",
    "a {\n  color: red;\n}\nc {\n  color: red;\n}\n"
);
test!(
    ambiguous_colon,
    ".btn {
        a:b {
            color: red;
        }
    }",
    ".btn a:b {\n  color: red;\n}\n"
);
test!(
    double_ampersand,
    "a {
        color: &&;
    }",
    "a {\n  color: a a;\n}\n"
);
test!(
    escaped_backslash_no_space_before_curly_brace,
    r#"\\{
        color: &;
    }"#,
    "\\\\ {\n  color: \\\\;\n}\n"
);
test!(
    pseudo_element_double_quotes,
    r#"::foo("red") {
        color: &;
    }"#,
    "::foo(\"red\") {\n  color: ::foo(\"red\");\n}\n"
);
