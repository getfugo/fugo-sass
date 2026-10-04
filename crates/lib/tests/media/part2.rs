use super::*;

test!(
    no_newline_between_media_rules_when_invisble_rule_between,
    "a {}

      @media (min-width: 5px) {
          a {
              color: 1;
          }
      }

      a {}

      @media (min-width: 5px) {
          a {
              color: 1;
          }
      }",
    "@media (min-width: 5px) {\n  a {\n    color: 1;\n  }\n}\n@media (min-width: 5px) {\n  a {\n    color: 1;\n  }\n}\n"
);
test!(
    two_media_rules_in_content_block,
    "@mixin foo() {
        @content;
    }

    @include foo {
        @media foo {
            a {
                color: red;
            }
        }
        @media foo {
            b {
                color: red;
            }
        }
    }",
    "@media foo {\n  a {\n    color: red;\n  }\n}\n@media foo {\n  b {\n    color: red;\n  }\n}\n"
);
test!(
    splits_child_nodes_when_preceding_media,
    "@media (foo) {
        @media (prefers-reduced-motion: reduce) {
            a {
                transition: none;
            }
        }

        a {
            color: red;
        }

        a {
            color: red;
        }
    }",
    "@media (foo) and (prefers-reduced-motion: reduce) {\n  a {\n    transition: none;\n  }\n}\n@media (foo) {\n  a {\n    color: red;\n  }\n}\n@media (foo) {\n  a {\n    color: red;\n  }\n}\n"
);
test!(
    doesnt_split_child_nodes_when_trailing_media,
    "@media (foo) {
        a {
            color: red;
        }

        a {
            color: red;
        }

        @media (prefers-reduced-motion: reduce) {
            a {
                transition: none;
            }
        }
    }",
    "@media (foo) {\n  a {\n    color: red;\n  }\n  a {\n    color: red;\n  }\n}\n@media (foo) and (prefers-reduced-motion: reduce) {\n  a {\n    transition: none;\n  }\n}\n"
);
test!(
    #[ignore = "our is_invisible_check inside css tree is flawed here"]
    doesnt_split_child_nodes_when_leading_but_invisible_media,
    "@media (foo) {
        @media (prefers-reduced-motion: reduce) {}

        a {
            color: red;
        }

        a {
            color: red;
        }
    }",
    "@media (foo) {\n  a {\n    color: red;\n  }\n  a {\n    color: red;\n  }\n}\n"
);
test!(
    media_has_url_in_parens,
    "@media (url) {
        a {
            color: red;
        }
    }",
    "@media (url) {\n  a {\n    color: red;\n  }\n}\n"
);
test!(
    #[ignore = "our is_invisible_check inside css tree is flawed here"]
    media_does_not_split_when_child_rule_has_invisible_media,
    "@media (min-width: 1px) {
        .first {
            font-weight: 100;

            @media (min-width: 2px) {}
        }

        .second {
            font-weight: 200;
        }
    }",
    "@media (min-width: 1px) {\n  .first {\n    font-weight: 100;\n  }\n  .second {\n    font-weight: 200;\n  }\n}\n"
);
test!(
    escaped_nullbyte_in_query,
    r#"@media (min-width:\0) {
        a {
            color: red;
        }
    }"#,
    "@media (min-width: \\0 ) {\n  a {\n    color: red;\n  }\n}\n"
);
test!(
    simple_unmergeable,
    "a {
        @media a {
            @media b {
                color: red;
            }
        }
    }",
    ""
);
test!(
    query_is_identifier_and_not_parens,
    "@media screen and not (foo) {
        a {
            color: red;
        }
    }",
    "@media screen and not (foo) {\n  a {\n    color: red;\n  }\n}\n"
);
test!(
    query_is_identifier_identifier_and_parens,
    "@media only screen and (foo) {
        a {
            color: red;
        }
    }",
    "@media only screen and (foo) {\n  a {\n    color: red;\n  }\n}\n"
);
test!(
    query_is_paren_and_paren,
    "@media (foo) and (bar) {
        a {
            color: red;
        }
    }",
    "@media (foo) and (bar) {\n  a {\n    color: red;\n  }\n}\n"
);
test!(
    query_is_paren_or_paren,
    "@media (foo) or (bar) {
        a {
            color: red;
        }
    }",
    "@media (foo) or (bar) {\n  a {\n    color: red;\n  }\n}\n"
);
error!(
    media_query_has_quoted_closing_paren,
    r#"@media ('a)'w) {
        a {
            color: red;
        }
    }"#,
    "Error: expected no more input."
);
error!(
    empty_query_after_resolving_interpolation,
    "@media #{null} {}", "Error: Expected identifier."
);
