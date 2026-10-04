use super::*;

test!(empty_extend_self, "a { @extend a; }", "");
test!(
    extend_self_with_styles,
    "a {\n  color: red;\n  @extend a;\n}\n",
    "a {\n  color: red;\n}\n"
);
test!(
    list_extends_both_of_compound,
    ".foo.bar {
        a: b
      }

      .x, .y {
        @extend .foo, .bar;
      }
    ",
    ".foo.bar, .x, .y {\n  a: b;\n}\n"
);
test!(
    class_extends_class_placed_second,
    ".foo {a: b}
     .bar {@extend .foo}",
    ".foo, .bar {\n  a: b;\n}\n"
);
test!(
    class_extends_class_placed_first,
    ".bar {@extend .foo}
    .foo {a: b}",
    ".foo, .bar {\n  a: b;\n}\n"
);
test!(
    class_extends_class_style_before_extend,
    ".foo {a: b}
     .bar {c: d; @extend .foo;}",
    ".foo, .bar {\n  a: b;\n}\n\n.bar {\n  c: d;\n}\n"
);
test!(
    class_extends_class_style_after_extend,
    ".foo {a: b}
     .bar {@extend .foo; c: d;}",
    ".foo, .bar {\n  a: b;\n}\n\n.bar {\n  c: d;\n}\n"
);
test!(
    class_extends_class_applies_to_multiple_extendees,
    ".foo {a: b}
    .bar {@extend .foo}
    .blip .foo {c: d}",
    ".foo, .bar {\n  a: b;\n}\n\n.blip .foo, .blip .bar {\n  c: d;\n}\n"
);
test!(
    class_extends_class_one_class_extends_multiple,
    ".foo {a: b}
    .bar {c: d}
    .baz {@extend .foo; @extend .bar}",
    ".foo, .baz {\n  a: b;\n}\n\n.bar, .baz {\n  c: d;\n}\n"
);
test!(
    class_extends_class_multiple_classes_extend_one,
    ".foo {a: b}
    .bar {@extend .foo}
    .baz {@extend .bar}
    .bip {@extend .bar}
    ",
    ".foo, .bar, .bip, .baz {\n  a: b;\n}\n"
);
test!(
    class_extends_class_all_parts_of_complex_selector_extended_by_one,
    ".foo .bar {a: b}
    .baz {@extend .foo; @extend .bar}
    ",
    ".foo .bar, .foo .baz, .baz .bar, .baz .baz {\n  a: b;\n}\n"
);
test!(
    class_extends_class_all_parts_of_compound_selector_extended_by_one,
    ".foo.bar {a: b}
    .baz {@extend .foo; @extend .bar}
    ",
    ".foo.bar, .baz {\n  a: b;\n}\n"
);
test!(
    class_extends_class_all_parts_of_complex_selector_extended_by_different,
    ".foo .bar {a: b}
    .baz {@extend .foo}
    .bang {@extend .bar}
    ",
    ".foo .bar, .foo .bang, .baz .bar, .baz .bang {\n  a: b;\n}\n"
);
test!(
    class_extends_class_all_parts_of_compound_selector_extended_by_different,
    ".foo.bar {a: b}
    .baz {@extend .foo}
    .bang {@extend .bar}
    ",
    ".foo.bar, .foo.bang, .bar.baz, .baz.bang {\n  a: b;\n}\n"
);
test!(
    class_extends_class_simple_selector_extended_chain,
    ".foo {a: b}
    .bar {@extend .foo}
    .baz {@extend .bar}
    .bip {@extend .bar}
    ",
    ".foo, .bar, .bip, .baz {\n  a: b;\n}\n"
);
test!(
    class_extends_class_interpolated,
    ".foo {a: b}
    .bar {@extend #{\".foo\"}}
    ",
    ".foo, .bar {\n  a: b;\n}\n"
);
test!(
    class_extends_class_target_child_of_complex,
    ".foo .bar {a: b}
    .baz {@extend .bar}
    ",
    ".foo .bar, .foo .baz {\n  a: b;\n}\n"
);
test!(
    class_extends_class_target_parent_of_complex,
    ".foo .bar {a: b}
    .baz {@extend .foo}
    ",
    ".foo .bar, .baz .bar {\n  a: b;\n}\n"
);
test!(
    class_unification_1,
    "%-a .foo.bar {a: b}
    .baz {@extend .foo} -a {@extend %-a}
    ",
    "-a .foo.bar, -a .bar.baz {\n  a: b;\n}\n"
);
test!(
    class_unification_2,
    "%-a .foo.baz {a: b}
    .baz {@extend .foo} -a {@extend %-a}
    ",
    "-a .baz {\n  a: b;\n}\n"
);
test!(
    id_unification_1,
    "%-a .foo.bar {a: b}
    #baz {@extend .foo} -a {@extend %-a}
    ",
    "-a .foo.bar, -a .bar#baz {\n  a: b;\n}\n"
);
test!(
    id_unification_2,
    "%-a .foo#baz {a: b}
    #baz {@extend .foo} -a {@extend %-a}
    ",
    "-a #baz {\n  a: b;\n}\n"
);
test!(
    universal_unification_simple_target_1,
    "%-a .foo {a: b}
    * {@extend .foo} -a {@extend %-a}
    ",
    "-a .foo, -a * {\n  a: b;\n}\n"
);
test!(
    universal_unification_simple_target_2,
    "%-a .foo.bar {a: b}
    * {@extend .foo} -a {@extend %-a}
    ",
    "-a .bar {\n  a: b;\n}\n"
);
test!(
    universal_unification_simple_target_3,
    "%-a .foo.bar {a: b}
    *|* {@extend .foo} -a {@extend %-a}
    ",
    "-a .bar {\n  a: b;\n}\n"
);
test!(
    universal_unification_simple_target_4,
    "%-a .foo.bar {a: b}
    ns|* {@extend .foo} -a {@extend %-a}
    ",
    "-a .foo.bar, -a ns|*.bar {\n  a: b;\n}\n"
);
test!(
    universal_unification_universal_target_without_namespace_1,
    "%-a *.foo {a: b}
    * {@extend .foo} -a {@extend %-a}
    ",
    "-a * {\n  a: b;\n}\n"
);
test!(
    universal_unification_universal_target_without_namespace_2,
    "%-a *.foo {a: b}
    *|* {@extend .foo} -a {@extend %-a}
    ",
    "-a * {\n  a: b;\n}\n"
);
test!(
    universal_unification_universal_target_without_namespace_3,
    "%-a *|*.foo {a: b}
    * {@extend .foo} -a {@extend %-a}
    ",
    "-a *|*.foo, -a * {\n  a: b;\n}\n"
);
test!(
    universal_unification_universal_target_without_namespace_4,
    "%-a *|*.foo {a: b}
    *|* {@extend .foo} -a {@extend %-a}
    ",
    "-a *|* {\n  a: b;\n}\n"
);
test!(
    universal_unification_universal_target_without_namespace_5,
    "%-a *.foo {a: b}
    ns|* {@extend .foo} -a {@extend %-a}
    ",
    "-a *.foo {\n  a: b;\n}\n"
);
test!(
    universal_unification_universal_target_without_namespace_6,
    "%-a *|*.foo {a: b}
    ns|* {@extend .foo} -a {@extend %-a}
    ",
    "-a *|*.foo, -a ns|* {\n  a: b;\n}\n"
);
test!(
    universal_unification_universal_target_without_namespace_7,
    "%-a ns|*.foo {a: b}
    * {@extend .foo} -a {@extend %-a}
    ",
    "-a ns|*.foo {\n  a: b;\n}\n"
);
test!(
    universal_unification_universal_target_without_namespace_8,
    "%-a ns|*.foo {a: b}
    *|* {@extend .foo} -a {@extend %-a}
    ",
    "-a ns|* {\n  a: b;\n}\n"
);
test!(
    universal_unification_universal_target_without_namespace_9,
    "%-a ns|*.foo {a: b}
    ns|* {@extend .foo} -a {@extend %-a}
    ",
    "-a ns|* {\n  a: b;\n}\n"
);
test!(
    universal_unification_element_target_without_namespace_1,
    "%-a a.foo {a: b}
    *|* {@extend .foo} -a {@extend %-a}
    ",
    "-a a {\n  a: b;\n}\n"
);
test!(
    universal_unification_element_target_without_namespace_2,
    "%-a *|a.foo {a: b}
    * {@extend .foo} -a {@extend %-a}
    ",
    "-a *|a.foo, -a a {\n  a: b;\n}\n"
);
test!(
    universal_unification_element_target_without_namespace_3,
    "%-a *|a.foo {a: b}
    *|* {@extend .foo} -a {@extend %-a}
    ",
    "-a *|a {\n  a: b;\n}\n"
);
test!(
    universal_unification_element_target_without_namespace_4,
    "%-a a.foo {a: b}
    ns|* {@extend .foo} -a {@extend %-a}
    ",
    "-a a.foo {\n  a: b;\n}\n"
);
test!(
    universal_unification_element_target_without_namespace_5,
    "%-a *|a.foo {a: b}
    ns|* {@extend .foo} -a {@extend %-a}
    ",
    "-a *|a.foo, -a ns|a {\n  a: b;\n}\n"
);
test!(
    universal_unification_element_target_without_namespace_6,
    "%-a ns|a.foo {a: b}
    * {@extend .foo} -a {@extend %-a}
    ",
    "-a ns|a.foo {\n  a: b;\n}\n"
);
test!(
    universal_unification_element_target_without_namespace_7,
    "%-a ns|a.foo {a: b}
    *|* {@extend .foo} -a {@extend %-a}
    ",
    "-a ns|a {\n  a: b;\n}\n"
);
test!(
    universal_unification_element_target_without_namespace_8,
    "%-a ns|a.foo {a: b}
    ns|* {@extend .foo} -a {@extend %-a}
    ",
    "-a ns|a {\n  a: b;\n}\n"
);
test!(
    element_unification_simple_target_1,
    "%-a .foo {a: b}
    a {@extend .foo} -a {@extend %-a}
    ",
    "-a .foo, -a a {\n  a: b;\n}\n"
);
test!(
    element_unification_simple_target_2,
    "%-a .foo.bar {a: b}
    a {@extend .foo} -a {@extend %-a}
    ",
    "-a .foo.bar, -a a.bar {\n  a: b;\n}\n"
);
test!(
    element_unification_simple_target_3,
    "%-a .foo.bar {a: b}
    *|a {@extend .foo} -a {@extend %-a}
    ",
    "-a .foo.bar, -a *|a.bar {\n  a: b;\n}\n"
);
test!(
    element_unification_simple_target_4,
    "%-a .foo.bar {a: b}
    ns|a {@extend .foo} -a {@extend %-a}
    ",
    "-a .foo.bar, -a ns|a.bar {\n  a: b;\n}\n"
);
test!(
    element_unification_universal_without_namespace_1,
    "%-a *.foo {a: b}
    a {@extend .foo} -a {@extend %-a}
    ",
    "-a *.foo, -a a {\n  a: b;\n}\n"
);
test!(
    element_unification_universal_without_namespace_2,
    "%-a *.foo {a: b}
    *|a {@extend .foo} -a {@extend %-a}
    ",
    "-a *.foo, -a a {\n  a: b;\n}\n"
);
test!(
    element_unification_universal_without_namespace_3,
    "%-a *|*.foo {a: b}
    a {@extend .foo} -a {@extend %-a}
    ",
    "-a *|*.foo, -a a {\n  a: b;\n}\n"
);
test!(
    element_unification_universal_without_namespace_4,
    "%-a *|*.foo {a: b}
    *|a {@extend .foo} -a {@extend %-a}
    ",
    "-a *|*.foo, -a *|a {\n  a: b;\n}\n"
);
test!(
    element_unification_universal_without_namespace_5,
    "%-a *.foo {a: b}
    ns|a {@extend .foo} -a {@extend %-a}
    ",
    "-a *.foo {\n  a: b;\n}\n"
);
test!(
    element_unification_universal_without_namespace_6,
    "%-a *|*.foo {a: b}
    ns|a {@extend .foo} -a {@extend %-a}
    ",
    "-a *|*.foo, -a ns|a {\n  a: b;\n}\n"
);
test!(
    element_unification_universal_with_namespace_1,
    "%-a ns|*.foo {a: b}
    a {@extend .foo} -a {@extend %-a}
    ",
    "-a ns|*.foo {\n  a: b;\n}\n"
);
test!(
    element_unification_universal_with_namespace_2,
    "%-a ns|*.foo {a: b}
    *|a {@extend .foo} -a {@extend %-a}
    ",
    "-a ns|*.foo, -a ns|a {\n  a: b;\n}\n"
);
test!(
    element_unification_universal_with_namespace_3,
    "%-a ns|*.foo {a: b}
    ns|a {@extend .foo} -a {@extend %-a}
    ",
    "-a ns|*.foo, -a ns|a {\n  a: b;\n}\n"
);
test!(
    element_unification_element_without_namespace_1,
    "%-a a.foo {a: b}
    a {@extend .foo} -a {@extend %-a}
    ",
    "-a a {\n  a: b;\n}\n"
);
test!(
    element_unification_element_without_namespace_2,
    "%-a a.foo {a: b}
    *|a {@extend .foo} -a {@extend %-a}
    ",
    "-a a {\n  a: b;\n}\n"
);
test!(
    element_unification_element_without_namespace_3,
    "%-a *|a.foo {a: b}
    a {@extend .foo} -a {@extend %-a}
    ",
    "-a *|a.foo, -a a {\n  a: b;\n}\n"
);
test!(
    element_unification_element_without_namespace_4,
    "%-a *|a.foo {a: b}
    *|a {@extend .foo} -a {@extend %-a}
    ",
    "-a *|a {\n  a: b;\n}\n"
);
test!(
    element_unification_element_without_namespace_5,
    "%-a a.foo {a: b}
    ns|a {@extend .foo} -a {@extend %-a}
    ",
    "-a a.foo {\n  a: b;\n}\n"
);
test!(
    element_unification_element_without_namespace_6,
    "%-a *|a.foo {a: b}
    ns|a {@extend .foo} -a {@extend %-a}
    ",
    "-a *|a.foo, -a ns|a {\n  a: b;\n}\n"
);
test!(
    element_unification_element_with_namespace_1,
    "%-a ns|a.foo {a: b}
    a {@extend .foo} -a {@extend %-a}
    ",
    "-a ns|a.foo {\n  a: b;\n}\n"
);
test!(
    element_unification_element_with_namespace_2,
    "%-a ns|a.foo {a: b}
    *|a {@extend .foo} -a {@extend %-a}
    ",
    "-a ns|a {\n  a: b;\n}\n"
);
