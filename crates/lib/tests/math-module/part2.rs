use super::*;

test!(
    hypot_negative,
    "@use 'sass:math';\na {\n  color: math.hypot(1px, 2px, 3px, 4px, 5px, -20px);\n}\n",
    "a {\n  color: 21.3307290077px;\n}\n"
);
test!(
    hypot_all_different_but_comparable_unit,
    "@use 'sass:math';\na {\n  color: math.hypot(1in, 2cm, 3mm, 4pt, 5pc);\n}\n",
    "a {\n  color: 1.5269191636in;\n}\n"
);
test!(
    hypot_all_no_unit,
    "@use 'sass:math';\na {\n  color: math.hypot(1, 2, 3);\n}\n",
    "a {\n  color: 3.7416573868;\n}\n"
);
test!(
    hypot_nan_has_comparable_unit,
    "@use 'sass:math';\na {\n  color: math.hypot(1deg, 2deg, math.acos(2));\n}\n",
    "a {\n  color: calc(NaN * 1deg);\n}\n"
);
error!(
    hypot_no_args,
    "@use 'sass:math';\na {\n  color: math.hypot();\n}\n",
    "Error: At least one argument must be passed."
);
error!(
    hypot_first_has_no_unit_third_has_unit,
    "@use 'sass:math';\na {\n  color: math.hypot(1, 2, 3px);\n}\n",
    "Error: Argument 1 is unitless but argument 3 has unit px. Arguments must all have units or all be unitless."
);
error!(
    hypot_non_numeric_argument,
    "@use 'sass:math';\na {\n  color: math.hypot(1, red, 3);\n}\n", "Error: red is not a number."
);
error!(
    hypot_units_not_comparable,
    "@use 'sass:math';\na {\n  color: math.hypot(1px, 2in, 3rem);\n}\n",
    "Error: Incompatible units px and rem."
);
error!(
    hypot_nan_has_no_unit_but_first_has_unit,
    "@use 'sass:math';\na {\n  color: math.hypot(1deg, 2deg, (0 / 0));\n}\n",
    "Error: Argument 1 has unit deg but argument 3 is unitless. Arguments must all have units or all be unitless."
);
test!(
    atan2_both_positive,
    "@use 'sass:math';\na {\n  color: math.atan2(3, 4);\n}\n",
    "a {\n  color: 36.8698976458deg;\n}\n"
);
test!(
    atan2_first_negative,
    "@use 'sass:math';\na {\n  color: math.atan2(-3, 4);\n}\n",
    "a {\n  color: -36.8698976458deg;\n}\n"
);
test!(
    atan2_second_negative,
    "@use 'sass:math';\na {\n  color: math.atan2(3, -4);\n}\n",
    "a {\n  color: 143.1301023542deg;\n}\n"
);
test!(
    atan2_both_negative,
    "@use 'sass:math';\na {\n  color: math.atan2(-3, -4);\n}\n",
    "a {\n  color: -143.1301023542deg;\n}\n"
);
test!(
    atan2_first_positive_second_zero,
    "@use 'sass:math';\na {\n  color: math.atan2(3, 0);\n}\n",
    "a {\n  color: 90deg;\n}\n"
);
test!(
    atan2_first_negative_second_zero,
    "@use 'sass:math';\na {\n  color: math.atan2(-3, 0);\n}\n",
    "a {\n  color: -90deg;\n}\n"
);
test!(
    atan2_first_zero_second_positive,
    "@use 'sass:math';\na {\n  color: math.atan2(0, 4);\n}\n",
    "a {\n  color: 0deg;\n}\n"
);
test!(
    atan2_first_zero_second_negative,
    "@use 'sass:math';\na {\n  color: math.atan2(0, -4);\n}\n",
    "a {\n  color: 180deg;\n}\n"
);
test!(
    atan2_both_zero,
    "@use 'sass:math';\na {\n  color: math.atan2(0, 0);\n}\n",
    "a {\n  color: 0deg;\n}\n"
);
test!(
    atan2_both_same_unit,
    "@use 'sass:math';\na {\n  color: math.atan2(3px, 4px);\n}\n",
    "a {\n  color: 36.8698976458deg;\n}\n"
);
test!(
    atan2_both_different_but_comparable_unit,
    "@use 'sass:math';\na {\n  color: math.atan2(3px, 4in);\n}\n",
    "a {\n  color: 0.4476141709deg;\n}\n"
);
error!(
    atan2_first_unitless_second_unit,
    "@use 'sass:math';\na {\n  color: math.atan2(3, 4rem);\n}\n",
    "Error: $y is unitless but $x has unit rem. Arguments must all have units or all be unitless."
);
error!(
    atan2_first_unit_second_unitless,
    "@use 'sass:math';\na {\n  color: math.atan2(3px, 4);\n}\n",
    "Error: $y has unit px but $x is unitless. Arguments must all have units or all be unitless."
);
error!(
    atan2_incompatible_units,
    "@use 'sass:math';\na {\n  color: math.atan2(3px, 4rem);\n}\n",
    "Error: Incompatible units px and rem."
);
error!(
    atan2_nan_incompatible_units,
    "@use 'sass:math';\na {\n  color: math.atan2(math.acos(2), 3);\n}\n",
    "Error: $y has unit deg but $x is unitless. Arguments must all have units or all be unitless."
);
test!(
    atan2_first_nan,
    "@use 'sass:math';\na {\n  color: math.atan2((0/0), 0);\n}\n",
    "a {\n  color: calc(NaN * 1deg);\n}\n"
);
test!(
    atan2_second_nan,
    "@use 'sass:math';\na {\n  color: math.atan2(0, (0/0));\n}\n",
    "a {\n  color: calc(NaN * 1deg);\n}\n"
);
test!(
    atan2_both_nan,
    "@use 'sass:math';\na {\n  color: math.atan2((0/0), (0/0));\n}\n",
    "a {\n  color: calc(NaN * 1deg);\n}\n"
);
test!(
    atan2_nan_with_same_units,
    "@use 'sass:math';\na {\n  color: math.atan2(math.acos(2), 3deg);\n}\n",
    "a {\n  color: calc(NaN * 1deg);\n}\n"
);
test!(
    div_two_integers,
    "@use 'sass:math';\na {\n  color: math.div(1, 2);\n}\n",
    "a {\n  color: 0.5;\n}\n"
);
test!(
    clamp_nan,
    "@use 'sass:math';\na {\n  color: math.clamp((0/0), 5, (0/0));\n}\n",
    "a {\n  color: 5;\n}\n"
);
test!(
    div_two_strings,
    "@use 'sass:math';\na {\n  color: math.div(\"1\",\"2\");\n}\n",
    "a {\n  color: \"1\"/\"2\";\n}\n"
);
test!(
    cos_nan,
    "@use 'sass:math';\na {\n  color: math.cos((0/0));\n}\n",
    "a {\n  color: calc(NaN);\n}\n"
);
test!(
    sin_nan,
    "@use 'sass:math';\na {\n  color: math.sin((0/0));\n}\n",
    "a {\n  color: calc(NaN);\n}\n"
);
test!(
    tan_nan,
    "@use 'sass:math';\na {\n  color: math.tan((0/0));\n}\n",
    "a {\n  color: calc(NaN);\n}\n"
);
test!(
    log_returns_whole_number_for_simple_base,
    "@use 'sass:math';
    a {
      color: math.log(8, 2);
      color: math.floor(math.log(8, 2));
    }",
    "a {\n  color: 3;\n  color: 3;\n}\n"
);

// todo: atan+asin with unitful NaN
