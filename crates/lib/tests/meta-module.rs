use std::io::Write;

use macros::TestFs;

#[macro_use]
mod macros;

test!(
    module_functions_builtin,
    "@use 'sass:meta';\na {\n  color: inspect(meta.module-functions(meta));\n}\n",
    "a {\n  color: (\"feature-exists\": get-function(\"feature-exists\"), \"inspect\": get-function(\"inspect\"), \"type-of\": get-function(\"type-of\"), \"keywords\": get-function(\"keywords\"), \"global-variable-exists\": get-function(\"global-variable-exists\"), \"variable-exists\": get-function(\"variable-exists\"), \"function-exists\": get-function(\"function-exists\"), \"mixin-exists\": get-function(\"mixin-exists\"), \"content-exists\": get-function(\"content-exists\"), \"module-variables\": get-function(\"module-variables\"), \"module-functions\": get-function(\"module-functions\"), \"get-function\": get-function(\"get-function\"), \"call\": get-function(\"call\"), \"calc-args\": get-function(\"calc-args\"), \"calc-name\": get-function(\"calc-name\"));\n}\n"
);
test!(
    module_variables_builtin,
    "@use 'sass:meta';\n@use 'sass:math';\na {\n  color: inspect(map-get(meta.module-variables(math), 'e'));\n}\n",
    "a {\n  color: 2.718281828459045;\n}\n"
);
test!(
    global_var_exists_module,
    "@use 'sass:math';\na {\n  color: global-variable-exists(pi, $module: math);\n}\n",
    "a {\n  color: true;\n}\n"
);
test!(
    fn_exists_builtin,
    "@use 'sass:math';\na {\n  color: function-exists(acos, $module: math);\n}\n",
    "a {\n  color: true;\n}\n"
);
error!(
    fn_exists_module_dne,
    "a {\n  color: function-exists(c, d);\n}\n",
    "Error: There is no module with the namespace \"d\"."
);

#[test]
fn mixin_exists_module() {
    let input = "@use \"mixin_exists_module\" as module;\na {\n color: mixin-exists(foo, $module: module);\n}";
    tempfile!("mixin_exists_module.scss", "@mixin foo {}");
    assert_eq!(
        "a {\n  color: true;\n}\n",
        &fugo_sass::from_string(input.to_string(), &fugo_sass::Options::default()).expect(input)
    );
}

#[test]
fn load_css_simple() {
    let input = "@use \"sass:meta\";\na {\n @include meta.load-css(load_css_simple);\n}";
    tempfile!("load_css_simple.scss", "a { color: red; }");
    assert_eq!(
        "a a {\n  color: red;\n}\n",
        &fugo_sass::from_string(input.to_string(), &fugo_sass::Options::default()).expect(input)
    );
}

#[test]
fn load_css_explicit_args() {
    let input = "@use \"sass:meta\";\na {\n @include meta.load-css($module: load_css_explicit_args, $with: null);\n}";
    tempfile!("load_css_explicit_args.scss", "a { color: red; }");
    assert_eq!(
        "a a {\n  color: red;\n}\n",
        &fugo_sass::from_string(input.to_string(), &fugo_sass::Options::default()).expect(input)
    );
}

#[test]
fn load_css_non_string_url() {
    let input = "@use \"sass:meta\";\na {\n @include meta.load-css(2);\n}";
    tempfile!("load_css_non_string_url.scss", "a { color: red; }");
    assert_err!("Error: $module: 2 is not a string.", input);
}

#[test]
fn load_css_non_map_with() {
    let input = "@use \"sass:meta\";\na {\n @include meta.load-css(foo, 2);\n}";
    assert_err!("Error: $with: 2 is not a map.", input);
}

#[test]
fn module_members_and_keywords_in_definition_order() {
    // Names are interned per thread, and these maps were ordered by the names' intern ids, so
    // names a previous compilation on the thread met first (here in reverse) came first.
    fugo_sass::from_string(
        "a { $c: 1; $b: 2; $a: 3; $x: 4; $y: 5; $z: 6; }".to_owned(),
        &fugo_sass::Options::default(),
    )
    .unwrap();
    let mut fs = TestFs::new();
    fs.add_file(
        "_vars.scss",
        "$a: 1; $b: 2; $c: 3;
        @function a() { @return 1; }
        @function c() { @return 1; }
        @function b() { @return 1; }",
    );
    let input = r#"
        @use "sass:map";
        @use "sass:meta";
        @use "vars";
        @function keys($args...) { @return map.keys(meta.keywords($args)); }
        a {
          variables: map.keys(meta.module-variables(vars));
          functions: map.keys(meta.module-functions(vars));
          keywords: keys($z: 1, $y: 2, $x: 3);
        }
    "#;
    assert_eq!(
        "a {\n  variables: \"a\", \"b\", \"c\";\n  functions: \"a\", \"c\", \"b\";\n  keywords: z, y, x;\n}\n",
        &fugo_sass::from_string(input.to_owned(), &fugo_sass::Options::default().fs(&fs))
            .expect(input)
    );
}
