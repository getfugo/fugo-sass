use super::*;

test!(
    placeholder_with_multiple_extenders,
    "%foo {a: b}
    .bar {@extend %foo}
    .baz {@extend %foo}
    ",
    ".baz, .bar {\n  a: b;\n}\n"
);
test!(
    placeholder_interpolation,
    "$foo: foo;

    %#{$foo} {a: b}
    .bar {@extend %foo}
    ",
    ".bar {\n  a: b;\n}\n"
);
test!(
    media_inside_placeholder,
    "%foo {bar {@media screen {a {b: c}}}}
    .baz {c: d}
    ",
    ".baz {\n  c: d;\n}\n"
);
test!(
    extend_within_media,
    "@media screen {
        .foo {a: b}
        .bar {@extend .foo}
    }
    ",
    "@media screen {\n  .foo, .bar {\n    a: b;\n  }\n}\n"
);
test!(
    extend_within_unknown_at_rule,
    "@unknown {
        .foo {a: b}
        .bar {@extend .foo}
    }
    ",
    "@unknown {\n  .foo, .bar {\n    a: b;\n  }\n}\n"
);
test!(
    extend_within_nested_at_rules,
    "@media screen {
      @unknown {
        .foo {a: b}
        .bar {@extend .foo}
      }
    }
    ",
    "@media screen {\n  @unknown {\n    .foo, .bar {\n      a: b;\n    }\n  }\n}\n"
);
test!(
    extend_within_separate_media_queries,
    "@media screen {.foo {a: b}}
    @media screen {.bar {@extend .foo}}
    ",
    "@media screen {\n  .foo, .bar {\n    a: b;\n  }\n}\n"
);
test!(
    extend_within_separate_unknown_at_rules,
    "@unknown {.foo {a: b}}
    @unknown {.bar {@extend .foo}}
    ",
    "@unknown {\n  .foo, .bar {\n    a: b;\n  }\n}\n@unknown {}\n"
);
test!(
    extend_within_separate_nested_at_rules,
    "@media screen {@flooblehoof {.foo {a: b}}}
     @media screen {@flooblehoof {.bar {@extend .foo}}}",
    "@media screen {\n  @flooblehoof {\n    .foo, .bar {\n      a: b;\n    }\n  }\n}\n@media screen {\n  @flooblehoof {}\n}\n"
);
test!(
    extend_succeeds_when_one_extend_fails_but_others_dont,
    "a.bar {a: b}
    .bar {c: d}
    b.foo {@extend .bar}
    ",
    "a.bar {\n  a: b;\n}\n\n.bar, b.foo {\n  c: d;\n}\n"
);
test!(
    optional_extend_succeeds_when_extendee_doesnt_exist,
    ".foo {@extend .bar !optional}",
    ""
);
test!(
    optional_extend_succeeds_when_extension_fails,
    "a.bar {a: b}
    b.foo {@extend .bar !optional}
    ",
    "a.bar {\n  a: b;\n}\n"
);
test!(
    psuedo_element_superselector_1,
    "%x#bar {a: b} // Add an id to make the results have high specificity
    %y, %y::fblthp {@extend %x}
    a {@extend %y}
    ",
    "a#bar, a#bar::fblthp {\n  a: b;\n}\n"
);
test!(
    psuedo_element_superselector_2,
    "%x#bar {a: b}
    %y, %y:fblthp {@extend %x}
    a {@extend %y}    
    ",
    "a#bar {\n  a: b;\n}\n"
);
test!(
    psuedo_element_superselector_3,
    "%x#bar {a: b}
    %y, %y:first-line {@extend %x}
    a {@extend %y}       
    ",
    "a#bar, a#bar:first-line {\n  a: b;\n}\n"
);
test!(
    psuedo_element_superselector_4,
    "%x#bar {a: b}
    %y, %y:first-letter {@extend %x}
    a {@extend %y}    
    ",
    "a#bar, a#bar:first-letter {\n  a: b;\n}\n"
);
test!(
    psuedo_element_superselector_5,
    "%x#bar {a: b}
    %y, %y:before {@extend %x}
    a {@extend %y}    
    ",
    "a#bar, a#bar:before {\n  a: b;\n}\n"
);
test!(
    psuedo_element_superselector_6,
    "%x#bar {a: b}
    %y, %y:after {@extend %x}
    a {@extend %y}    
    ",
    "a#bar, a#bar:after {\n  a: b;\n}\n"
);
test!(
    multiple_source_redundancy_elimination,
    "%default-color {color: red}
    %alt-color {color: green}
    
    %default-style {
    @extend %default-color;
    &:hover {@extend %alt-color}
    &:active {@extend %default-color}
    }
    
    .test-case {@extend %default-style}    
    ",
    ".test-case:active, .test-case {\n  color: red;\n}\n\n.test-case:hover {\n  color: green;\n}\n"
);
test!(
    nested_sibling_extend,
    ".foo {@extend .bar}

    .parent {
      .bar {
        a: b;
      }
      .foo {
        @extend .bar
      }
    }    
    ",
    ".parent .bar, .parent .foo {\n  a: b;\n}\n"
);
test!(
    parent_and_sibling_extend,
    "%foo %bar%baz {a: b}

    .parent1 {
      @extend %foo;
      .child1 {@extend %bar}
    }
    
    .parent2 {
      @extend %foo;
      .child2 {@extend %baz}
    }    
    ",
    ".parent1 .parent2 .child1.child2, .parent2 .parent1 .child1.child2 {\n  a: b;\n}\n"
);
test!(
    nested_extend_specificity,
    "%foo {a: b}

    a {
      :b {@extend %foo}
      :b:c {@extend %foo}
    }
    ",
    "a :b:c, a :b {\n  a: b;\n}\n"
);
test!(
    double_extend_optimization,
    "%foo %bar {
        a: b;
    }
        
    .parent1 {
        @extend %foo;
        
        .child {
          @extend %bar;
        }
    }
        
    .parent2 {
        @extend %foo;
    }        
    ",
    ".parent1 .child {\n  a: b;\n}\n"
);
test!(
    extend_inside_double_nested_media,
    "@media all {
        @media (orientation: landscape) {
          %foo {color: blue}
          .bar {@extend %foo}
        }
    }        
    ",
    "@media (orientation: landscape) {\n  .bar {\n    color: blue;\n  }\n}\n"
);
test!(
    partially_failed_extend,
    "test { @extend .rc; }
    .rc {color: white;}
    .prices span.pill span.rc {color: red;}    
    ",
    ".rc, test {\n  color: white;\n}\n\n.prices span.pill span.rc {\n  color: red;\n}\n"
);
test!(
    newline_near_combinator,
    ".a +
    .b x {a: b}
    .c y {@extend x}    
    ",
    ".a + .b x, .a + .b .c y, .c .a + .b y {\n  a: b;\n}\n"
);
test!(
    duplicated_selector_with_newlines,
    ".example-1-1,
    .example-1-2,
    .example-1-3 {
      a: b;
    }
    
    .my-page-1 .my-module-1-1 {@extend .example-1-2}     
    ",
    ".example-1-1,\n.example-1-2,\n.my-page-1 .my-module-1-1,\n.example-1-3 {\n  a: b;\n}\n"
);
test!(
    nested_selector_with_child_selector_hack_extendee,
    "> .foo {a: b}
    foo bar {@extend .foo}    
    ",
    "> .foo, > foo bar {\n  a: b;\n}\n"
);
test!(
    nested_selector_with_child_selector_hack_extender,
    ".foo .bar {a: b}
    > foo bar {@extend .bar}    
    ",
    ".foo .bar, > .foo foo bar, > foo .foo bar {\n  a: b;\n}\n"
);
test!(
    nested_selector_with_child_selector_hack_extender_and_extendee,
    "> .foo {a: b}
    > foo bar {@extend .foo}    
    ",
    "> .foo, > foo bar {\n  a: b;\n}\n"
);
test!(
    nested_selector_with_child_selector_hack_extender_and_sibling_extendee,
    "~ .foo {a: b}
    > foo bar {@extend .foo}    
    ",
    "~ .foo {\n  a: b;\n}\n"
);
test!(
    nested_selector_with_child_selector_hack_extender_and_extendee_newline,
    "> .foo {a: b}\nflip,\n> foo bar {@extend .foo}\n",
    "> .foo, > flip,\n> foo bar {\n  a: b;\n}\n"
);
test!(
    extended_parent_and_child_redundancy_elimination,
    "a {
        b {a: b}
        c {@extend b}
    }
    d {@extend a}
    ",
    "a b, d b, a c, d c {\n  a: b;\n}\n"
);
test!(
    redundancy_elimination_when_it_would_reduce_specificity,
    "a {a: b}
    a.foo {@extend a}    
    ",
    "a, a.foo {\n  a: b;\n}\n"
);
test!(
    redundancy_elimination_when_it_would_preserve_specificity,
    ".bar a {a: b}
    a.foo {@extend a}    
    ",
    ".bar a {\n  a: b;\n}\n"
);
test!(
    redundancy_elimination_never_eliminates_base_selector,
    "a.foo {a: b}
    .foo {@extend a}      
    ",
    "a.foo, .foo {\n  a: b;\n}\n"
);
test!(
    cross_branch_redundancy_elimination_1,
    "%x .c %y {a: b}
    .a, .b {@extend %x}
    .a .d {@extend %y}    
    ",
    ".a .c .d, .b .c .a .d {\n  a: b;\n}\n"
);
test!(
    cross_branch_redundancy_elimination_2,
    ".e %z {a: b}
    %x .c %y {@extend %z}
    .a, .b {@extend %x}
    .a .d {@extend %y}    
    ",
    ".e .a .c .d, .e .b .c .a .d, .a .e .b .c .d, .a .c .e .d, .b .c .e .a .d {\n  a: b;\n}\n"
);
test!(
    extend_with_universal_selector,
    "%-a *.foo1 {a: b}
    a {@extend .foo1}
    -a {@extend %-a}
    
    %-b *|*.foo2 {b: b}
    b {@extend .foo2}
    -b {@extend %-b}    
    ",
    "-a *.foo1, -a a {\n  a: b;\n}\n\n-b *|*.foo2, -b b {\n  b: b;\n}\n"
);
test!(
    extend_with_universal_selector_empty_namespace,
    "%-a |*.foo {a: b}
    a {@extend .foo}
    -a {@extend %-a}    
    ",
    "-a |*.foo {\n  a: b;\n}\n"
);
test!(
    extend_with_universal_selector_different_namespace,
    "%-a ns|*.foo {a: b}
    a {@extend .foo}
    -a {@extend %-a}    
    ",
    "-a ns|*.foo {\n  a: b;\n}\n"
);
test!(
    unify_root_pseudo_element,
    "// We assume that by default classes don't apply to the :root unless marked explicitly.
    :root .foo-1 { test: 1; }
    .bar-1 .baz-1 { @extend .foo-1; }
    
    // We know the two classes must be the same :root element so we can combine them.
    .foo-2:root .bar-2 { test: 2; }
    .baz-2:root .bang-2 { @extend .bar-2; }
    
    // This extend should not apply because the :root elements are different.
    html:root .bar-3 { test: 3; }
    xml:root .bang-3 { @extend .bar-3}
    
    // We assume that direct descendant of the :root is not the same element as a descendant.
    .foo-4:root > .bar-4 .x-4 { test: 4; }
    .baz-4:root .bang-4 .y-4 {@extend .x-4}    
    ",
    ":root .foo-1, :root .bar-1 .baz-1 {\n  test: 1;\n}\n\n.foo-2:root .bar-2, .baz-2.foo-2:root .bang-2 {\n  test: 2;\n}\n\nhtml:root .bar-3 {\n  test: 3;\n}\n\n.foo-4:root > .bar-4 .x-4, .baz-4.foo-4:root > .bar-4 .bang-4 .y-4 {\n  test: 4;\n}\n"
);
test!(
    compound_unification_in_not,
    "// Make sure compound selectors are unified when two :not()s are extended.
    // :not() is special here because it's the only selector that's extended by
    // adding to the compound selector, rather than creating a new selector list.
    .a {@extend .c}
    .b {@extend .d}
    :not(.c):not(.d) {a: b}    
    ",
    ":not(.c):not(.a):not(.d):not(.b) {\n  a: b;\n}\n"
);
test!(
    does_not_move_page_block_in_media,
    "@media screen {
        a { x:y; }
        @page {}
    }      
    ",
    "@media screen {\n  a {\n    x: y;\n  }\n  @page {}\n}\n"
);
test!(
    escaped_selector,
    "// Escapes in selectors' identifiers should be normalized before `@extend` is
    // applied.
    .foo {escape: none}
    \\.foo {escape: slash dot}
    \\2E foo {escape: hex}
    
    .bar {@extend \\02e foo}    
    ",
    ".foo {\n  escape: none;\n}\n\n\\.foo, .bar {\n  escape: slash dot;\n}\n\n\\.foo, .bar {\n  escape: hex;\n}\n"
);
test!(
    extend_extender,
    "// For implementations like Dart Sass that process extensions as they occur,
    // extending rules that contain their own extends needs special handling.
    .b {@extend .a}
    .c {@extend .b}
    .a {x: y}
    ",
    ".a, .b, .c {\n  x: y;\n}\n"
);
test!(
    extend_result_of_extend,
    "// The result of :not(.c) being extended should itself be extendable.
    .a {@extend :not(.b)}
    .b {@extend .c}
    :not(.c) {x: y}    
    ",
    ":not(.c):not(.b), .a:not(.c) {\n  x: y;\n}\n"
);
test!(
    extend_self,
    "// This shouldn't change the selector.
    .c, .a .b .c, .a .c .b {x: y; @extend .c}    
    ",
    ".c, .a .b .c, .a .c .b {\n  x: y;\n}\n"
);
