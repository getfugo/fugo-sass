use super::*;

test!(
    dart_sass_issue_146,
    "%btn-style-default {
        background: green;
        &:hover{
          background: black;
        }
      }
      
      button {
        @extend %btn-style-default;
      }      
    ",
    "button {\n  background: green;\n}\nbutton:hover {\n  background: black;\n}\n"
);
test!(
    nested_compound_unification,
    "// Make sure compound unification properly handles weaving together parent
    // selectors.
    .a .b {@extend .e}
    .c .d {@extend .f}
    .e.f {x: y}    
    ",
    ".e.f, .a .f.b, .c .e.d, .a .c .b.d, .c .a .b.d {\n  x: y;\n}\n"
);
test!(
    not_into_not_not,
    "// Regression test for dart-sass#191.
    :not(:not(.x)) {a: b}
    :not(.y) {@extend .x}    
    ",
    ":not(:not(.x)) {\n  a: b;\n}\n"
);
test!(
    selector_list,
    ".foo {a: b}
    .bar {x: y}
    
    // Extending a selector list is equivalent to writing two @extends.
    .baz {@extend .foo, .bar}
    
    // The selector list should be parsed after interpolation is resolved.
    .bang {@extend .foo #{\",\"} .bar}    
    ",
    ".foo, .bang, .baz {\n  a: b;\n}\n\n.bar, .bang, .baz {\n  x: y;\n}\n"
);
test!(
    selector_list_after_selector,
    "a {
        color: red;
    }

    b,
    c {
        @extend a;
    }",
    "a, b,\nc {\n  color: red;\n}\n"
);
test!(
    selector_list_before_selector,
    "b, c {
        @extend a;
    }

    a {
        color: red;
    }",
    "a, b, c {\n  color: red;\n}\n"
);
test!(
    selector_list_of_selector_pseudo_classes_after_selector,
    "foo {
        color: black;
    }

    a:current(foo),
    :current(foo) {
        @extend foo;
    }",
    "foo, a:current(foo),\n:current(foo) {\n  color: black;\n}\n"
);
test!(
    extend_pseudo_selector_class_containing_combinator_without_rhs_selector,
    ":has(a >) b {
        @extend b;
        color: red;
    }",
    ":has(a >) b, :has(a >) :has(a >) :has(a >) b, :has(a >) :has(a >) :has(a >) b {\n  color: red;\n}\n"
);
test!(
    extend_after_target,
    ".a .b {
        c: d;
      }
      
      .a.mod1, .a.mod2 {
        @extend .a, .b;
      }
      .a.mod3, .a.mod4 {
        @extend .a, .b;
      }
      .a.mod5, .a.mod6 {
        @extend .a, .b;
      }",
    ".a .b, .a .a.mod5, .a .a.mod6, .a .a.mod3, .a .a.mod4, .a .a.mod1, .a .a.mod2 {\n  c: d;\n}\n"
);
test!(
    parent_selector_as_value_ignores_extend,
    "a {
      color: &;
    }

    b {
      @extend a;
    }",
    "a, b {\n  color: a;\n}\n"
);
test!(
    complex_selector_with_combinator_removed_by_complex_selector_without_combinator,
    "c b {
      @extend %d;
    }
    
    c > b {
      @extend %d;
    }
    
    %d {
      color: red;
    }",
    "c b {\n  color: red;\n}\n"
);
test!(
    unification_subselector_of_target_where,
    r#"a {b: selector-extend(".c:where(d)", ":where(d)", "d.e")}"#,
    "a {\n  b: .c:where(d);\n}\n"
);
error!(
    extend_optional_keyword_not_complete,
    "a {
        @extend a !opt;
    }",
    "Error: Expected \"optional\"."
);
error!(
    extend_contains_parent_in_compound_selector,
    "a {
        @extend &b:c; 
    }",
    "Error: Parent selectors aren't allowed here."
);
error!(
    #[ignore = "we do not currently respect this"]
    extend_across_media_boundary,
    "a {
        display: none;
    }

    @media only screen and (min-width:300px) {
        a {
            @extend a;
        }
    }",
    "Error: You may not @extend selectors across media queries."
);
error!(
    #[ignore = "we do not error for this"]
    extend_target_does_not_exist,
    "a {
        @extend dne;
    }",
    "Error: The target selector was not found."
);
error!(
    #[ignore = "crash"]
    extends_self_is_has_invalid_combinator,
    "a :is(#a, >) {
        @extend a
    }",
    ""
);
error!(
    extend_complex_selector,
    "a {
        @extend a>b;
    }",
    "Error: complex selectors may not be extended."
);
error!(
    extend_at_root_of_document,
    "@extend a;", "Error: @extend may only be used within style rules."
);

// todo: extend_loop (massive test)
// todo: extend tests in folders
// todo: copy all :where extend tests, https://github.com/sass/sass-spec/pull/1783/files

#[test]
fn extend_reaches_every_rule_of_a_selector() {
    // Style rules' selectors were hashed by address but compared by value, so of two rules with
    // the same selector the second lost its extensions whenever the random hasher made their
    // hashes meet: a few of these 200 rules on every run.
    let mut input: String = (0..200)
        .map(|i| format!(".a {{ order: {i}; }}\n"))
        .collect();
    input.push_str(".b { @extend .a; }\n");
    let css = fugo_sass::from_string(
        input,
        &fugo_sass::Options::default().style(fugo_sass::OutputStyle::Compressed),
    )
    .unwrap();
    assert_eq!(css.matches(".a,.b{").count(), 200, "{css}");
}
error!(
    target_not_found,
    "a {@extend .missing;}", "Error: The target selector was not found."
);
