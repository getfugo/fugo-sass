use super::*;

test!(
    can_access_variables_declared_before_content,
    "@mixin foo {
        $a: red;

        @content;

        color: $a;
    }

    a {
      @include foo;
    }",
    "a {\n  color: red;\n}\n"
);
test!(
    content_contains_variable_declared_in_outer_scope_not_declared_at_root,
    "a {
        $a: red;

        @mixin foo {
            @content;
        }

        @include foo {
            color: $a;
        }
    }",
    "a {\n  color: red;\n}\n"
);
test!(
    content_contains_variable_declared_in_outer_scope_declared_at_root,
    "@mixin foo {
        @content;
    }

    a {
        $a: red;

        @include foo {
            color: $a;
        }
    }",
    "a {\n  color: red;\n}\n"
);
test!(
    content_contains_variable_declared_in_outer_scope_not_declared_at_root_and_modified,
    "a {
        $a: wrong;

        @mixin foo {
            $a: correct;
            @content;
        }

        @include foo {
            color: $a;
        }
    }",
    "a {\n  color: correct;\n}\n"
);
test!(
    content_contains_variable_declared_in_outer_scope_declared_at_root_and_modified,
    "@mixin foo {
        $a: wrong;
        @content;
    }

    a {
        $a: correct;


        @include foo {
            color: $a;
        }
    }",
    "a {\n  color: correct;\n}\n"
);
test!(
    content_default_arg_value_no_parens,
    "a {
        @mixin foo {
            @content;
        }

        @include foo using ($a: red) {
            color: $a;
        }
    }",
    "a {\n  color: red;\n}\n"
);
test!(
    space_between_content_and_args,
    "space-after-content {
        @mixin mixin {
            @content /**/ (value1, value2);
        }

        @include mixin using ($arg1, $arg2) {
            arg1: $arg1;
            arg2: $arg2;
        }
    }",
    "space-after-content {\n  arg1: value1;\n  arg2: value2;\n}\n"
);
test!(
    space_between_mixin_and_args,
    "@mixin foo /**/ ()  /**/ {
        color: red;
    }

    a {
        @include foo;
    }",
    "a {\n  color: red;\n}\n"
);
test!(
    mixin_cant_affect_scope_in_which_it_was_included,
    "@mixin test {
        $a: wrong;
    }

    a {
        $a: correct;
        @include test;
        color: $a;
    }",
    "a {\n  color: correct;\n}\n"
);
test!(
    content_block_has_two_rulesets,
    "@mixin foo() {
        @content;
    }

    @include foo {
        a {
            color: red;
        }

        b {
            color: red;
        }
    }",
    "a {\n  color: red;\n}\n\nb {\n  color: red;\n}\n"
);
test!(
    mixin_has_two_rulesets,
    "@mixin foo() {
        a {
            display: none;
        }

        b {
            display: block;
        }
    }

    @include foo();",
    "a {\n  display: none;\n}\n\nb {\n  display: block;\n}\n"
);
test!(
    sass_spec__188_test_mixin_content,
    "$color: blue;

    @mixin context($class, $color: red) {
        .#{$class} {
            background-color: $color;
            @content;
            border-color: $color;
        }
    }

    @include context(parent) {
        @include context(child, $color: yellow) {
            color: $color;
        }
    }",
    ".parent {\n  background-color: red;\n}\n.parent .child {\n  background-color: yellow;\n  color: blue;\n  border-color: yellow;\n}\n.parent {\n  border-color: red;\n}\n"
);
test!(
    sass_spec__mixin_environment_locality,
    r#"// The "$var" variable should only be set locally, despite being in the same
    // mixin each time.
    @mixin with-local-variable($recurse) {
        $var: before;

        @if ($recurse) {
            @include with-local-variable($recurse: false);
        }

        var: $var;
        $var: after;
    }

    .environment-locality {
        @include with-local-variable($recurse: true);
    }"#,
    ".environment-locality {\n  var: before;\n  var: before;\n}\n"
);
test!(
    parses_extend_inside_mixin_not_in_style_rule,
    "@mixin foo {
        @extend a;
    }",
    ""
);
error!(
    parses_extend_inside_content_block_not_in_style_rule,
    "@mixin foo {
        @content;
    }

    @include foo {
        @extend a;
    }",
    "Error: @extend may only be used within style rules."
);
error!(
    mixin_in_function,
    "@function foo() {
        @mixin bar {}
    }
    a {
        color: foo();
    }
    ",
    "Error: This at-rule is not allowed here."
);
error!(
    mixin_in_mixin,
    "@mixin foo {
        @mixin bar {}
    }
    a {
        @include foo;
    }
    ",
    "Error: Mixins may not contain mixin declarations."
);
error!(
    mixin_in_control_directives,
    "@if true {
        @mixin bar {}
    }",
    "Error: Mixins may not be declared in control directives."
);
error!(
    does_not_allow_interpolation_in_name_of_declaration,
    "@mixin n#{a}me {
        color: red;
    }

    a {
        @include name;
    }",
    "Error: expected \"{\"."
);
error!(
    disallows_content_block_when_mixin_has_no_content_block,
    "@mixin foo () {}
    @include foo {}
    ",
    "Error: Mixin doesn't accept a content block."
);
error!(
    disallows_content_block_to_builtin_mixin,
    r#"@use "sass:meta";

    @include meta.load-css("") {}"#,
    "Error: Mixin doesn't accept a content block."
);
error!(
    disallows_interpolation_in_include_name,
    r#"@mixin foo {}
    @include f#{o}o;"#,
    "Error: expected \";\"."
);
