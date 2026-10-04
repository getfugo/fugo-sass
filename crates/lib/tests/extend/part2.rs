use super::*;

test!(
    element_unification_element_with_namespace_3,
    "%-a ns|a.foo {a: b}
    ns|a {@extend .foo} -a {@extend %-a}
    ",
    "-a ns|a {\n  a: b;\n}\n"
);
test!(
    attribute_unification_1,
    "%-a [foo=bar].baz {a: b}
    [foo=baz] {@extend .baz} -a {@extend %-a}
    ",
    "-a [foo=bar].baz, -a [foo=bar][foo=baz] {\n  a: b;\n}\n"
);
test!(
    attribute_unification_2,
    "%-a [foo=bar].baz {a: b}
    [foo^=bar] {@extend .baz} -a {@extend %-a}
    ",
    "-a [foo=bar].baz, -a [foo=bar][foo^=bar] {\n  a: b;\n}\n"
);
test!(
    attribute_unification_3,
    "%-a [foo=bar].baz {a: b}
    [foot=bar] {@extend .baz} -a {@extend %-a}
    ",
    "-a [foo=bar].baz, -a [foo=bar][foot=bar] {\n  a: b;\n}\n"
);
test!(
    attribute_unification_4,
    "%-a [foo=bar].baz {a: b}
    [ns|foo=bar] {@extend .baz} -a {@extend %-a}
    ",
    "-a [foo=bar].baz, -a [foo=bar][ns|foo=bar] {\n  a: b;\n}\n"
);
test!(
    attribute_unification_5,
    "%-a %-a [foo=bar].bar {a: b}
    [foo=bar] {@extend .bar} -a {@extend %-a}
    ",
    "-a -a [foo=bar] {\n  a: b;\n}\n"
);
test!(
    pseudo_unification_1,
    "%-a :foo.baz {a: b}
    :foo(2n+1) {@extend .baz} -a {@extend %-a}
    ",
    "-a :foo.baz, -a :foo:foo(2n+1) {\n  a: b;\n}\n"
);
test!(
    pseudo_unification_2,
    "%-a :foo.baz {a: b}
    ::foo {@extend .baz} -a {@extend %-a}
    ",
    "-a :foo.baz, -a :foo::foo {\n  a: b;\n}\n"
);
test!(
    pseudo_unification_3,
    "%-a ::foo.baz {a: b}
    ::foo {@extend .baz} -a {@extend %-a}
    ",
    "-a ::foo {\n  a: b;\n}\n"
);
test!(
    pseudo_unification_4,
    "%-a ::foo(2n+1).baz {a: b}
    ::foo(2n+1) {@extend .baz} -a {@extend %-a}
    ",
    "-a ::foo(2n+1) {\n  a: b;\n}\n"
);
test!(
    pseudo_unification_5,
    "%-a :foo.baz {a: b}
    :bar {@extend .baz} -a {@extend %-a}
    ",
    "-a :foo.baz, -a :foo:bar {\n  a: b;\n}\n"
);
test!(
    pseudo_unification_6,
    "%-a .baz:foo {a: b}
    :after {@extend .baz} -a {@extend %-a}
    ",
    "-a .baz:foo, -a :foo:after {\n  a: b;\n}\n"
);
test!(
    pseudo_unification_7,
    "%-a .baz:after {a: b}
    :foo {@extend .baz} -a {@extend %-a}
    ",
    "-a .baz:after, -a :foo:after {\n  a: b;\n}\n"
);
test!(
    pseudo_unification_8,
    "%-a :foo.baz {a: b}
    :foo {@extend .baz} -a {@extend %-a}
    ",
    "-a :foo {\n  a: b;\n}\n"
);
test!(
    pseudoelement_remains_at_end_of_selector_1,
    ".foo::bar {a: b}
    .baz {@extend .foo}
    ",
    ".foo::bar, .baz::bar {\n  a: b;\n}\n"
);
test!(
    pseudoelement_remains_at_end_of_selector_2,
    "a.foo::bar {a: b}
    .baz {@extend .foo}
    ",
    "a.foo::bar, a.baz::bar {\n  a: b;\n}\n"
);
test!(
    pseudoclass_remains_at_end_of_selector_1,
    ".foo:bar {a: b}
    .baz {@extend .foo}
    ",
    ".foo:bar, .baz:bar {\n  a: b;\n}\n"
);
test!(
    pseudoclass_remains_at_end_of_selector_2,
    "a.foo:bar {a: b}
    .baz {@extend .foo}
    ",
    "a.foo:bar, a.baz:bar {\n  a: b;\n}\n"
);
test!(
    pseudoclass_not_remains_at_end_of_selector,
    ".foo:not(.bar) {a: b}
    .baz {@extend .foo}
    ",
    ".foo:not(.bar), .baz:not(.bar) {\n  a: b;\n}\n"
);
test!(
    pseudoelement_goes_lefter_than_pseudoclass_1,
    ".foo::bar {a: b}
    .baz:bang {@extend .foo}
    ",
    ".foo::bar, .baz:bang::bar {\n  a: b;\n}\n"
);
test!(
    pseudoelement_goes_lefter_than_pseudoclass_2,
    ".foo:bar {a: b}
    .baz::bang {@extend .foo}
    ",
    ".foo:bar, .baz:bar::bang {\n  a: b;\n}\n"
);
test!(
    pseudoelement_goes_lefter_than_not_1,
    ".foo::bar {a: b}
    .baz:not(.bang) {@extend .foo}
    ",
    ".foo::bar, .baz:not(.bang)::bar {\n  a: b;\n}\n"
);
test!(
    pseudoelement_goes_lefter_than_not_2,
    "%a {
        a:b;
      }
      b:after:not(:first-child) {
        @extend %a;
      }
      c:s {
        @extend %a;
      }
      d::e {
        @extend c;
      }
    ",
    "c:s, d:s::e, b:after:not(:first-child) {\n  a: b;\n}\n"
);
test!(
    pseudoelement_goes_lefter_than_not_3,
    ".foo:not(.bang) {a: b}
    .baz::bar {@extend .foo}
    ",
    ".foo:not(.bang), .baz:not(.bang)::bar {\n  a: b;\n}\n"
);
test!(
    negation_unification_1,
    "%-a :not(.foo).baz {a: b}
    :not(.bar) {@extend .baz} -a {@extend %-a}
    ",
    "-a :not(.foo).baz, -a :not(.foo):not(.bar) {\n  a: b;\n}\n"
);
test!(
    negation_unification_2,
    "%-a :not(.foo).baz {a: b}
    :not(.foo) {@extend .baz} -a {@extend %-a}
    ",
    "-a :not(.foo) {\n  a: b;\n}\n"
);
test!(
    negation_unification_3,
    "%-a :not([a=b]).baz {a: b}
    :not([a = b]) {@extend .baz} -a {@extend %-a}
    ",
    "-a :not([a=b]) {\n  a: b;\n}\n"
);
test!(
    comma_extendee,
    ".foo {a: b}
    .bar {c: d}
    .baz {@extend .foo, .bar}
    ",
    ".foo, .baz {\n  a: b;\n}\n\n.bar, .baz {\n  c: d;\n}\n"
);
test!(
    redundant_selector_elimination,
    ".foo.bar {a: b}
    .x {@extend .foo, .bar}
    .y {@extend .foo, .bar}
    ",
    ".foo.bar, .y, .x {\n  a: b;\n}\n"
);
error!(
    extend_compound_selector,
    "ns|*.foo.bar {a: b}
    a.baz {@extend .foo.bar}
    ",
    "Error: compound selectors may no longer be extended."
);
test!(
    compound_extender,
    ".foo.bar {a: b}
    .baz.bang {@extend .foo}
    ",
    ".foo.bar, .bar.baz.bang {\n  a: b;\n}\n"
);
test!(
    compound_extender_unification,
    "ns|*.foo.bar {a: b}
    a.baz {@extend .foo}
    ",
    "ns|*.foo.bar {\n  a: b;\n}\n"
);
test!(
    complex_extender,
    ".foo {a: b}
    foo bar {@extend .foo}
    ",
    ".foo, foo bar {\n  a: b;\n}\n"
);
test!(
    complex_extender_unification,
    ".foo.bar {a: b}
    foo bar {@extend .foo}
    ",
    ".foo.bar, foo bar.bar {\n  a: b;\n}\n"
);
test!(
    complex_extender_alternates_parents,
    ".baz .bip .foo {a: b}
    foo .grank bar {@extend .foo}
    ",
    ".baz .bip .foo, .baz .bip foo .grank bar, foo .grank .baz .bip bar {\n  a: b;\n}\n"
);
test!(
    complex_extender_unifies_identical_parents,
    ".baz .bip .foo {a: b}
    .baz .bip bar {@extend .foo}
    ",
    ".baz .bip .foo, .baz .bip bar {\n  a: b;\n}\n"
);
test!(
    complex_extender_unifies_common_substring,
    ".baz .bip .bap .bink .foo {a: b}
    .brat .bip .bap bar {@extend .foo}
    ",
    ".baz .bip .bap .bink .foo, .baz .brat .bip .bap .bink bar, .brat .baz .bip .bap .bink bar {\n  a: b;\n}\n"
);
test!(
    complex_extender_unifies_common_subsequence,
    ".a .x .b .y .foo {a: b}
    .a .n .b .m bar {@extend .foo}
    ",
    ".a .x .b .y .foo, .a .x .n .b .y .m bar, .a .n .x .b .y .m bar, .a .x .n .b .m .y bar, .a .n .x .b .m .y bar {\n  a: b;\n}\n"
);
test!(
    complex_extender_chooses_first_subsequence,
    ".a .b .c .d .foo {a: b}
    .c .d .a .b .bar {@extend .foo}
    ",
    ".a .b .c .d .foo, .a .b .c .d .a .b .bar {\n  a: b;\n}\n"
);
test!(
    complex_extender_counts_extended_superselectors,
    ".a .bip .foo {a: b}
    .b .bip.bop .bar {@extend .foo}
    ",
    ".a .bip .foo, .a .b .bip.bop .bar, .b .a .bip.bop .bar {\n  a: b;\n}\n"
);
test!(
    complex_extender_child_combinator,
    ".baz .foo {a: b}
    foo > bar {@extend .foo}
    ",
    ".baz .foo, .baz foo > bar {\n  a: b;\n}\n"
);
test!(
    complex_extender_finds_common_selectors_around_child_combinator_1,
    "a > b c .c1 {a: b}
    a c .c2 {@extend .c1}
    ",
    "a > b c .c1, a > b c .c2 {\n  a: b;\n}\n"
);
test!(
    complex_extender_finds_common_selectors_around_child_combinator_2,
    "a > b c .c1 {a: b}
    b c .c2 {@extend .c1}
    ",
    "a > b c .c1, a > b c .c2 {\n  a: b;\n}\n"
);
test!(
    complex_extender_finds_common_selectors_around_adjacent_sibling_combinator_1,
    "a + b c .c1 {a: b}
    a c .c2 {@extend .c1}
    ",
    "a + b c .c1, a + b a c .c2, a a + b c .c2 {\n  a: b;\n}\n"
);
test!(
    complex_extender_finds_common_selectors_around_adjacent_sibling_combinator_2,
    "a + b c .c1 {a: b}
    a b .c2 {@extend .c1}
    ",
    "a + b c .c1, a a + b c .c2 {\n  a: b;\n}\n"
);
test!(
    complex_extender_finds_common_selectors_around_adjacent_sibling_combinator_3,
    "a + b c .c1 {a: b}
    b c .c2 {@extend .c1}
    ",
    "a + b c .c1, a + b c .c2 {\n  a: b;\n}\n"
);
test!(
    complex_extender_finds_common_selectors_around_sibling_combinator_1,
    "a ~ b c .c1 {a: b}
    a c .c2 {@extend .c1}
    ",
    "a ~ b c .c1, a ~ b a c .c2, a a ~ b c .c2 {\n  a: b;\n}\n"
);
test!(
    complex_extender_finds_common_selectors_around_sibling_combinator_2,
    "a ~ b c .c1 {a: b}
    a b .c2 {@extend .c1}
    ",
    "a ~ b c .c1, a a ~ b c .c2 {\n  a: b;\n}\n"
);
test!(
    complex_extender_finds_common_selectors_around_sibling_combinator_3,
    "a ~ b c .c1 {a: b}
    b c .c2 {@extend .c1}
    ",
    "a ~ b c .c1, a ~ b c .c2 {\n  a: b;\n}\n"
);
test!(
    complex_extender_with_early_child_selectors_doesnt_subsequence_them_1,
    ".bip > .bap .foo {a: b}
    .grip > .bap .bar {@extend .foo}
    ",
    ".bip > .bap .foo, .bip > .bap .grip > .bap .bar, .grip > .bap .bip > .bap .bar {\n  a: b;\n}\n"
);
test!(
    complex_extender_with_early_child_selectors_doesnt_subsequence_them_2,
    ".bap > .bip .foo {a: b}
    .bap > .grip .bar {@extend .foo}
    ",
    ".bap > .bip .foo, .bap > .bip .bap > .grip .bar, .bap > .grip .bap > .bip .bar {\n  a: b;\n}\n"
);
test!(
    complex_extender_with_child_selector_unifies_1,
    ".baz.foo {a: b}
    foo > bar {@extend .foo}
    ",
    ".baz.foo, foo > bar.baz {\n  a: b;\n}\n"
);
test!(
    complex_extender_with_child_selector_unifies_2,
    ".baz > {
        .foo {a: b}
        .bar {@extend .foo}
    }
    ",
    ".baz > .foo, .baz > .bar {\n  a: b;\n}\n"
);
test!(
    complex_extender_with_child_selector_unifies_3,
    ".foo {
        .bar {a: b}
        > .baz {@extend .bar}
    }
    ",
    ".foo .bar, .foo > .baz {\n  a: b;\n}\n"
);
test!(
    complex_extender_with_early_child_selector_1,
    ".foo {
        .bar {a: b}
        .bip > .baz {@extend .bar}
    }
    ",
    ".foo .bar, .foo .bip > .baz {\n  a: b;\n}\n"
);
test!(
    complex_extender_with_early_child_selector_2,
    ".foo {
        .bip .bar {a: b}
        > .baz {@extend .bar}
    }
    ",
    ".foo .bip .bar, .foo .bip .foo > .baz {\n  a: b;\n}\n"
);
test!(
    complex_extender_with_early_child_selector_3,
    ".foo > .bar {a: b}
    .bip + .baz {@extend .bar}
    ",
    ".foo > .bar, .foo > .bip + .baz {\n  a: b;\n}\n"
);
test!(
    complex_extender_with_early_child_selector_4,
    ".foo + .bar {a: b}
    .bip > .baz {@extend .bar}
    ",
    ".foo + .bar, .bip > .foo + .baz {\n  a: b;\n}\n"
);
test!(
    complex_extender_with_early_child_selector_5,
    ".foo > .bar {a: b}
    .bip > .baz {@extend .bar}
    ",
    ".foo > .bar, .bip.foo > .baz {\n  a: b;\n}\n"
);
test!(
    complex_extender_with_sibling_selector,
    ".baz .foo {a: b}
    foo + bar {@extend .foo}
    ",
    ".baz .foo, .baz foo + bar {\n  a: b;\n}\n"
);
test!(
    complex_extender_with_hacky_selector_1,
    ".baz .foo {a: b}
    foo + > > + bar {@extend .foo}
    ",
    ".baz .foo, .baz foo + > > + bar, foo .baz + > > + bar {\n  a: b;\n}\n"
);
