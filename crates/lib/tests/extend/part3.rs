use super::*;

test!(
    complex_extender_with_hacky_selector_2,
    ".baz .foo {a: b}
    > > bar {@extend .foo}
    ",
    ".baz .foo, > > .baz bar {\n  a: b;\n}\n"
);
test!(
    complex_extender_merges_with_the_same_selector,
    ".foo {
        .bar {a: b}
        .baz {@extend .bar}
    }
    ",
    ".foo .bar, .foo .baz {\n  a: b;\n}\n"
);
test!(
    complex_extender_with_child_selector_merges_with_the_same_selector,
    ".foo > .bar .baz {a: b}
    .foo > .bar .bang {@extend .baz}
    ",
    ".foo > .bar .baz, .foo > .bar .bang {\n  a: b;\n}\n"
);
test!(
    combinator_unification_for_hacky_combinators_1,
    ".a > + x {a: b}
    .b y {@extend x}
    ",
    ".a > + x, .a .b > + y, .b .a > + y {\n  a: b;\n}\n"
);
test!(
    combinator_unification_for_hacky_combinators_2,
    ".a x {a: b}
    .b > + y {@extend x}
    ",
    ".a x, .a .b > + y, .b .a > + y {\n  a: b;\n}\n"
);
test!(
    combinator_unification_for_hacky_combinators_3,
    ".a > + x {a: b}
    .b > + y {@extend x}
    ",
    ".a > + x, .a .b > + y, .b .a > + y {\n  a: b;\n}\n"
);
test!(
    combinator_unification_for_hacky_combinators_4,
    ".a ~ > + x {a: b}
    .b > + y {@extend x}
    ",
    ".a ~ > + x, .a .b ~ > + y, .b .a ~ > + y {\n  a: b;\n}\n"
);
test!(
    combinator_unification_for_hacky_combinators_5,
    ".a + > x {a: b}
    .b > + y {@extend x}
    ",
    ".a + > x {\n  a: b;\n}\n"
);
test!(
    combinator_unification_for_hacky_combinators_6,
    ".a + > x {a: b}
    .b > + y {@extend x}
    ",
    ".a + > x {\n  a: b;\n}\n"
);
test!(
    combinator_unification_for_hacky_combinators_7,
    ".a ~ > + .b > x {a: b}
    .c > + .d > y {@extend x}
    ",
    ".a ~ > + .b > x, .a .c ~ > + .d.b > y, .c .a ~ > + .d.b > y {\n  a: b;\n}\n"
);
test!(
    combinator_unification_double_tilde_1,
    ".a.b ~ x {a: b}
    .a ~ y {@extend x}
    ",
    ".a.b ~ x, .a.b ~ y {\n  a: b;\n}\n"
);
test!(
    combinator_unification_double_tilde_2,
    ".a ~ x {a: b}
    .a.b ~ y {@extend x}
    ",
    ".a ~ x, .a.b ~ y {\n  a: b;\n}\n"
);
test!(
    combinator_unification_double_tilde_3,
    ".a ~ x {a: b}
    .b ~ y {@extend x}
    ",
    ".a ~ x, .a ~ .b ~ y, .b ~ .a ~ y, .b.a ~ y {\n  a: b;\n}\n"
);
test!(
    combinator_unification_double_tilde_4,
    "a.a ~ x {a: b}
    b.b ~ y {@extend x}
    ",
    "a.a ~ x, a.a ~ b.b ~ y, b.b ~ a.a ~ y {\n  a: b;\n}\n"
);
test!(
    combinator_unification_tilde_plus_1,
    ".a.b + x {a: b}
    .a ~ y {@extend x}
    ",
    ".a.b + x, .a.b + y {\n  a: b;\n}\n"
);
test!(
    combinator_unification_tilde_plus_2,
    ".a + x {a: b}
    .a.b ~ y {@extend x}
    ",
    ".a + x, .a.b ~ .a + y, .a.b + y {\n  a: b;\n}\n"
);
test!(
    combinator_unification_tilde_plus_3,
    ".a + x {a: b}
    .b ~ y {@extend x}
    ",
    ".a + x, .b ~ .a + y, .b.a + y {\n  a: b;\n}\n"
);
test!(
    combinator_unification_tilde_plus_4,
    "a.a + x {a: b}
    b.b ~ y {@extend x}
    ",
    "a.a + x, b.b ~ a.a + y {\n  a: b;\n}\n"
);
test!(
    combinator_unification_tilde_plus_5,
    ".a.b ~ x {a: b}
    .a + y {@extend x}
    ",
    ".a.b ~ x, .a.b ~ .a + y, .a.b + y {\n  a: b;\n}\n"
);
test!(
    combinator_unification_tilde_plus_6,
    ".a ~ x {a: b}
    .a.b + y {@extend x}
    ",
    ".a ~ x, .a.b + y {\n  a: b;\n}\n"
);
test!(
    combinator_unification_tilde_plus_7,
    ".a ~ x {a: b}
    .b + y {@extend x}
    ",
    ".a ~ x, .a ~ .b + y, .b.a + y {\n  a: b;\n}\n"
);
test!(
    combinator_unification_angle_sibling_1,
    ".a > x {a: b}
    .b ~ y {@extend x}
    ",
    ".a > x, .a > .b ~ y {\n  a: b;\n}\n"
);
test!(
    combinator_unification_angle_sibling_2,
    ".a > x {a: b}
    .b + y {@extend x}
    ",
    ".a > x, .a > .b + y {\n  a: b;\n}\n"
);
test!(
    combinator_unification_angle_sibling_3,
    ".a ~ x {a: b}
    .b > y {@extend x}
    ",
    ".a ~ x, .b > .a ~ y {\n  a: b;\n}\n"
);
test!(
    combinator_unification_angle_sibling_4,
    ".a + x {a: b}
    .b > y {@extend x}
    ",
    ".a + x, .b > .a + y {\n  a: b;\n}\n"
);
test!(
    combinator_unification_double_angle_1,
    ".a.b > x {a: b}
    .b > y {@extend x}
    ",
    ".a.b > x, .b.a > y {\n  a: b;\n}\n"
);
test!(
    combinator_unification_double_angle_2,
    ".a > x {a: b}
    .a.b > y {@extend x}
    ",
    ".a > x, .a.b > y {\n  a: b;\n}\n"
);
test!(
    combinator_unification_double_angle_3,
    ".a > x {a: b}
    .b > y {@extend x}
    ",
    ".a > x, .b.a > y {\n  a: b;\n}\n"
);
test!(
    combinator_unification_double_angle_4,
    "a.a > x {a: b}
    b.b > y {@extend x}
    ",
    "a.a > x {\n  a: b;\n}\n"
);
test!(
    combinator_unification_double_plus_1,
    ".a.b + x {a: b}
    .b + y {@extend x}
    ",
    ".a.b + x, .b.a + y {\n  a: b;\n}\n"
);
test!(
    combinator_unification_double_plus_2,
    ".a + x {a: b}
    .a.b + y {@extend x}
    ",
    ".a + x, .a.b + y {\n  a: b;\n}\n"
);
test!(
    combinator_unification_double_plus_3,
    ".a + x {a: b}
    .b + y {@extend x}
    ",
    ".a + x, .b.a + y {\n  a: b;\n}\n"
);
test!(
    combinator_unification_double_plus_4,
    "a.a + x {a: b}
    b.b + y {@extend x}
    ",
    "a.a + x {\n  a: b;\n}\n"
);
test!(
    combinator_unification_angle_space_1,
    ".a.b > x {a: b}
    .a y {@extend x}
    ",
    ".a.b > x, .a.b > y {\n  a: b;\n}\n"
);
test!(
    combinator_unification_angle_space_2,
    ".a > x {a: b}
    .a.b y {@extend x}
    ",
    ".a > x, .a.b .a > y {\n  a: b;\n}\n"
);
test!(
    combinator_unification_angle_space_3,
    ".a > x {a: b}
    .b y {@extend x}
    ",
    ".a > x, .b .a > y {\n  a: b;\n}\n"
);
test!(
    combinator_unification_angle_space_4,
    ".a.b x {a: b}
    .a > y {@extend x}
    ",
    ".a.b x, .a.b .a > y {\n  a: b;\n}\n"
);
test!(
    combinator_unification_angle_space_5,
    ".a x {a: b}
    .a.b > y {@extend x}
    ",
    ".a x, .a.b > y {\n  a: b;\n}\n"
);
test!(
    combinator_unification_angle_space_6,
    ".a x {a: b}
    .b > y {@extend x}
    ",
    ".a x, .a .b > y {\n  a: b;\n}\n"
);
test!(
    combinator_unification_plus_space_1,
    ".a.b + x {a: b}
    .a y {@extend x}
    ",
    ".a.b + x, .a .a.b + y {\n  a: b;\n}\n"
);
test!(
    combinator_unification_plus_space_2,
    ".a + x {a: b}
    .a.b y {@extend x}
    ",
    ".a + x, .a.b .a + y {\n  a: b;\n}\n"
);
test!(
    combinator_unification_plus_space_3,
    ".a + x {a: b}
    .b y {@extend x}
    ",
    ".a + x, .b .a + y {\n  a: b;\n}\n"
);
test!(
    combinator_unification_plus_space_4,
    ".a.b x {a: b}
    .a + y {@extend x}
    ",
    ".a.b x, .a.b .a + y {\n  a: b;\n}\n"
);
test!(
    combinator_unification_plus_space_5,
    ".a x {a: b}
    .a.b + y {@extend x}
    ",
    ".a x, .a .a.b + y {\n  a: b;\n}\n"
);
test!(
    combinator_unification_plus_space_6,
    ".a x {a: b}
    .b + y {@extend x}
    ",
    ".a x, .a .b + y {\n  a: b;\n}\n"
);
test!(
    nested_combinator_unification_1,
    ".a > .b + x {a: b}
    .c > .d + y {@extend x}
    ",
    ".a > .b + x, .c.a > .d.b + y {\n  a: b;\n}\n"
);
test!(
    nested_combinator_unification_2,
    ".a > .b + x {a: b}
    .c > y {@extend x}
    ",
    ".a > .b + x, .c.a > .b + y {\n  a: b;\n}\n"
);
test!(
    combinator_unification_with_newlines,
    ".a >\n.b\n+ x {a: b}\n.c\n> .d +\ny {@extend x}\n",
    ".a > .b + x, .c.a > .d.b + y {\n  a: b;\n}\n"
);
test!(
    basic_extend_loop,
    ".foo {a: b; @extend .bar}
    .bar {c: d; @extend .foo}
    ",
    ".foo, .bar {\n  a: b;\n}\n\n.bar, .foo {\n  c: d;\n}\n"
);
test!(
    three_level_extend_loop,
    ".foo {a: b; @extend .bar}
    .bar {c: d; @extend .baz}
    .baz {e: f; @extend .foo}
    ",
    ".foo, .baz, .bar {\n  a: b;\n}\n\n.bar, .foo, .baz {\n  c: d;\n}\n\n.baz, .bar, .foo {\n  e: f;\n}\n"
);
test!(
    nested_extend_loop,
    ".bar {
        a: b;
        .foo {c: d; @extend .bar}
    }
    ",
    ".bar, .bar .foo {\n  a: b;\n}\n.bar .foo {\n  c: d;\n}\n"
);
test!(
    multiple_extender_merges_with_superset_selector,
    ".foo {@extend .bar; @extend .baz}
    a.bar.baz {a: b}
    ",
    "a.bar.baz, a.foo {\n  a: b;\n}\n"
);
test!(
    inside_control_flow_if,
    ".true  { color: green; }
    .false { color: red;   }
    .also-true {
    @if true { @extend .true;  }
    @else    { @extend .false; }
    }
    .also-false {
    @if false { @extend .true;  }
    @else     { @extend .false; }
    }
    ",
    ".true, .also-true {\n  color: green;\n}\n\n.false, .also-false {\n  color: red;\n}\n"
);
test!(
    inside_control_flow_for,
    "
    .base-0  { color: green; }
    .base-1  { display: block; }
    .base-2  { border: 1px solid blue; }
    .added {
      @for $i from 0 to 3 {
          @extend .base-#{$i};
      }
    }
    ",
    ".base-0, .added {\n  color: green;\n}\n\n.base-1, .added {\n  display: block;\n}\n\n.base-2, .added {\n  border: 1px solid blue;\n}\n"
);
test!(
    inside_control_flow_while,
    "
    .base-0  { color: green; }
    .base-1  { display: block; }
    .base-2  { border: 1px solid blue; }
    .added {
      $i : 0;
      @while $i < 3 {
        @extend .base-#{$i};
        $i : $i + 1;
      }
    }
    ",
    ".base-0, .added {\n  color: green;\n}\n\n.base-1, .added {\n  display: block;\n}\n\n.base-2, .added {\n  border: 1px solid blue;\n}\n"
);
test!(
    basic_placeholder,
    "%foo {a: b}
    .bar {@extend %foo}
    ",
    ".bar {\n  a: b;\n}\n"
);
test!(
    unused_placeholder,
    "%foo {a: b}
    %bar {a: b}
    .baz {@extend %foo}
    ",
    ".baz {\n  a: b;\n}\n"
);
test!(
    placeholder_descendant,
    "#context %foo a {a: b}
    .bar {@extend %foo}
    ",
    "#context .bar a {\n  a: b;\n}\n"
);
test!(
    semi_placeholder,
    "#context %foo, .bar .baz {a: b}

    .bat {
      @extend %foo;
    }
    ",
    "#context .bat, .bar .baz {\n  a: b;\n}\n"
);
