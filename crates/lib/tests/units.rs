#[macro_use]
mod macros;

test!(
    unit_none,
    "a {\n  height: 1;\n}\n",
    "a {\n  height: 1;\n}\n"
);
test!(
    unit_not_attached,
    "a {\n  height: 1 px;\n}\n",
    "a {\n  height: 1 px;\n}\n"
);
test!(
    unit_px,
    "a {\n  height: 1px;\n}\n",
    "a {\n  height: 1px;\n}\n"
);
test!(
    unit_em,
    "a {\n  height: 1em;\n}\n",
    "a {\n  height: 1em;\n}\n"
);
test!(
    unit_rem,
    "a {\n  height: 1rem;\n}\n",
    "a {\n  height: 1rem;\n}\n"
);
test!(
    unit_percent,
    "a {\n  height: 1%;\n}\n",
    "a {\n  height: 1%;\n}\n"
);
test!(
    unit_times_none,
    "a {\n  color: 3px * 2;\n}\n",
    "a {\n  color: 6px;\n}\n"
);
test!(
    none_times_unit,
    "a {\n  color: 2 * 3px;\n}\n",
    "a {\n  color: 6px;\n}\n"
);
test!(
    unit_fn_unit_times_none,
    "a {\n  color: unit(1px * 1);\n}\n",
    "a {\n  color: \"px\";\n}\n"
);
test!(
    unit_fn_none_times_unit,
    "a {\n  color: unit(1 * 1px);\n}\n",
    "a {\n  color: \"px\";\n}\n"
);
test!(
    unit_fn_unit_times_unit,
    "a {\n  color: unit(1px*1px);\n}\n",
    "a {\n  color: \"px*px\";\n}\n"
);
test!(
    unit_fn_unit_times_unit_times_unit,
    "a {\n  color: unit(1px * 1rad * 1em);\n}\n",
    "a {\n  color: \"px*rad*em\";\n}\n"
);
test!(
    unit_none_times_none_times_none,
    "a {\n  color: 1 * 1 * 1;\n}\n",
    "a {\n  color: 1;\n}\n"
);
test!(
    unit_plus_none,
    "a {\n  color: 10px + 10;\n}\n",
    "a {\n  color: 20px;\n}\n"
);
test!(
    none_plus_unit,
    "a {\n  color: 10 + 10px;\n}\n",
    "a {\n  color: 20px;\n}\n"
);
test!(
    unit_minus_none,
    "a {\n  color: 10px - 10;\n}\n",
    "a {\n  color: 0px;\n}\n"
);
test!(
    none_minus_unit,
    "a {\n  color: 10 - 10px;\n}\n",
    "a {\n  color: 0px;\n}\n"
);
test!(
    percent_plus_none,
    "a {\n  color: 10% + 10;\n}\n",
    "a {\n  color: 20%;\n}\n"
);
test!(
    unit_no_hyphen,
    "a {\n  color: 1px-2px;\n}\n",
    "a {\n  color: -1px;\n}\n"
);
test!(
    unit_starts_with_escape_sequence,
    "a {\n  color: 1\\9;\n}\n",
    "a {\n  color: 1\\9 ;\n}\n"
);
test!(
    unit_fn_starts_with_escape_sequence,
    "a {\n  color: unit(1\\9);\n}\n",
    "a {\n  color: \"\\\\9 \";\n}\n"
);
test!(
    non_ascii_numeric_interpreted_as_unit,
    "a {\n  color: 2߄;\n}\n",
    "@charset \"UTF-8\";\na {\n  color: 2߄;\n}\n"
);
test!(
    unit_div_same,
    "a {\n  color: unit(1em / 1em);\n}\n",
    "a {\n  color: \"\";\n}\n"
);
test!(
    unit_div_first_none,
    "a {\n  color: unit(1 / 1em);\n}\n",
    "a {\n  color: \"em^-1\";\n}\n"
);
test!(
    unit_div_second_none,
    "a {\n  color: unit(1em / 1);\n}\n",
    "a {\n  color: \"em\";\n}\n"
);
test!(
    unit_div_comparable,
    "a {\n  color: unit(1in / 1px);\n  color: (1in / 1px);\n}\n",
    "a {\n  color: \"\";\n  color: 96;\n}\n"
);
test!(
    unit_mul_times_mul,
    "a {\n  color: unit((1em * 1px) * (1em * 1px));\n}\n",
    "a {\n  color: \"em*px*em*px\";\n}\n"
);
test!(
    unit_single_times_mul,
    "a {\n  color: unit(1in * (1em * 1px));\n}\n",
    "a {\n  color: \"in*em*px\";\n}\n"
);
test!(
    unit_div_lhs_mul_uncomparable,
    "a {\n  color: unit((1 / 1in) * 1em);\n}\n",
    "a {\n  color: \"em/in\";\n}\n"
);
test!(
    unit_div_lhs_mul_same,
    "a {\n  color: unit((1 / 1in) * 1in);\n}\n",
    "a {\n  color: \"\";\n}\n"
);
test!(
    unit_begins_with_single_hyphen,
    "a {\n  color: unit(1-em);\n}\n",
    "a {\n  color: \"-em\";\n}\n"
);
test!(
    unit_begins_with_two_hyphens,
    "a {\n  color: 1--em;\n}\n",
    "a {\n  color: 1 --em;\n}\n"
);
test!(
    unit_begins_with_escape_sequence,
    "a {\n  color: unit(1\\65);\n}\n",
    "a {\n  color: \"e\";\n}\n"
);
test!(
    unit_begins_with_escape_sequence_followed_by_space_and_hyphen,
    "a {\n  color: unit(1\\65 -);\n}\n",
    "a {\n  color: \"e-\";\n}\n"
);
test!(
    unit_begins_with_single_hyphen_followed_by_escape_sequence,
    "a {\n  color: unit(1-\\65);\n}\n",
    "a {\n  color: \"-e\";\n}\n"
);
test!(
    viewport_relative_comparable_same,
    "a {\n  color: comparable(1vw, 2vw);\n}\n",
    "a {\n  color: true;\n}\n"
);
test!(
    viewport_relative_comparable_different,
    "a {\n  color: comparable(1vw, 1vh);\n}\n",
    "a {\n  color: false;\n}\n"
);
test!(
    removes_same_unit_from_complex_in_division,
    "a {\n  color: ((1px*1px) / 1px);\n}\n",
    "a {\n  color: 1px;\n}\n"
);
test!(
    removes_comparable_unit_from_complex_in_division_and_does_conversion,
    "a {\n  color: ((1in*1in) / 1cm);\n}\n",
    "a {\n  color: 2.54in;\n}\n"
);
test!(
    add_complex_div_units,
    "a {\n  color: inspect((1em / 1em) + (1px / 1em));\n}\n",
    "a {\n  color: calc(2px / 1em);\n}\n"
);
test!(
    #[ignore = "we need to rewrite how we compare and convert units"]
    complex_units_with_same_denom_and_comparable_numer_are_comparable,
    "a {\n  color: comparable((23in/2fu), (23cm/2fu));\n}\n",
    "a {\n  color: true;\n}\n"
);
test!(
    complex_unit_many_denom_one_numer,
    "a {\n  color: unit((1rem/1px) / 1vh);\n}\n",
    "a {\n  color: \"rem/px*vh\";\n}\n"
);
test!(
    complex_unit_empty_numerator_single_denom,
    "a {\n  color: unit(1 / 1px);\n}\n",
    "a {\n  color: \"px^-1\";\n}\n"
);
test!(
    complex_unit_empty_numerator_multiple_denom,
    "a {\n  color: unit(1 / (1px*1rem));\n}\n",
    "a {\n  color: \"(px*rem)^-1\";\n}\n"
);
test!(
    display_single_div_with_none_numerator,
    "a {\n  color: (1 / 1em);\n}\n",
    "a {\n  color: calc(1 / 1em);\n}\n"
);
error!(
    // note: dart-sass has error "Error: 1X and 1dppx have incompatible units."
    capital_x_is_not_alias_for_dppx,
    "a {\n  color: 1X + 1dppx;\n}\n", "Error: Incompatible units dppx and X."
);
error!(
    // note: dart-sass has error "Error: 1x and 1dppx have incompatible units."
    lowercase_x_is_not_alias_for_dppx,
    "a {\n  color: 1x + 1dppx;\n}\n", "Error: Incompatible units dppx and x."
);
test!(
    display_single_div_with_non_comparable_numerator,
    "a {\n  color: (1px / 1em);\n}\n",
    "a {\n  color: calc(1px / 1em);\n}\n"
);
test!(
    display_single_mul,
    "a {\n  color: 1rem * 1px;\n}\n",
    "a {\n  color: calc(1rem * 1px);\n}\n"
);
test!(
    display_arbitrary_mul,
    "a {\n  color: 1rem * 1px * 1rad * 1foo;\n}\n",
    "a {\n  color: calc(1rem * 1px * 1rad * 1foo);\n}\n"
);
test!(
    display_single_div_with_none_numerator_percent,
    "a {\n  color: (35 / 7%);\n}\n",
    "a {\n  color: calc(5 / 1%);\n}\n"
);
test!(
    /// Verify the display implementation of all special-cased units
    render_units,
    "a {
        color: 1px;
        color: 1mm;
        color: 1in;
        color: 1cm;
        color: 1q;
        color: 1pt;
        color: 1pc;
        color: 1em;
        color: 1rem;
        color: 1lh;
        color: 1%;
        color: 1ex;
        color: 1ch;
        color: 1cap;
        color: 1ic;
        color: 1rlh;
        color: 1vw;
        color: 1vh;
        color: 1vmin;
        color: 1vmax;
        color: 1vi;
        color: 1vb;
        color: 1deg;
        color: 1grad;
        color: 1rad;
        color: 1turn;
        color: 1s;
        color: 1ms;
        color: 1Hz;
        color: 1kHz;
        color: 1dpi;
        color: 1dpcm;
        color: 1dppx;
        color: 1fr;
        color: 1foo;
    }", "a {\n  color: 1px;\n  color: 1mm;\n  color: 1in;\n  color: 1cm;\n  color: 1q;\n  color: 1pt;\n  color: 1pc;\n  color: 1em;\n  color: 1rem;\n  color: 1lh;\n  color: 1%;\n  color: 1ex;\n  color: 1ch;\n  color: 1cap;\n  color: 1ic;\n  color: 1rlh;\n  color: 1vw;\n  color: 1vh;\n  color: 1vmin;\n  color: 1vmax;\n  color: 1vi;\n  color: 1vb;\n  color: 1deg;\n  color: 1grad;\n  color: 1rad;\n  color: 1turn;\n  color: 1s;\n  color: 1ms;\n  color: 1Hz;\n  color: 1kHz;\n  color: 1dpi;\n  color: 1dpcm;\n  color: 1dppx;\n  color: 1fr;\n  color: 1foo;\n}\n"
);

// `test_unit_addition!(in_plus_cm, in, cm, "1.39...")` tests that `1in + 1cm` is `1.39...in`.
macro_rules! test_unit_addition {
    ($name:ident, $u1:ident, $u2:ident, $out:literal) => {
        test!(
            $name,
            concat!(
                "a {\n  color: 1",
                stringify!($u1),
                " + 1",
                stringify!($u2),
                ";\n}\n"
            ),
            format!("a {{\n  color: {}{};\n}}\n", $out, stringify!($u1))
        );
    };
}

test_unit_addition!(in_plus_in, in, in, "2");
test_unit_addition!(in_plus_cm, in, cm, "1.3937007874");
test_unit_addition!(in_plus_pc, in, pc, "1.1666666667");
test_unit_addition!(in_plus_mm, in, mm, "1.0393700787");
test_unit_addition!(in_plus_q, in, q, "1.0098425197");
test_unit_addition!(in_plus_pt, in, pt, "1.0138888889");
test_unit_addition!(in_plus_px, in, px, "1.0104166667");

test_unit_addition!(cm_plus_in, cm, in, "3.54");
test_unit_addition!(cm_plus_cm, cm, cm, "2");
test_unit_addition!(cm_plus_pc, cm, pc, "1.4233333333");
test_unit_addition!(cm_plus_mm, cm, mm, "1.1");
test_unit_addition!(cm_plus_q, cm, q, "1.025");
test_unit_addition!(cm_plus_pt, cm, pt, "1.0352777778");
test_unit_addition!(cm_plus_px, cm, px, "1.0264583333");

test_unit_addition!(pc_plus_in, pc, in, "7");
test_unit_addition!(pc_plus_cm, pc, cm, "3.3622047244");
test_unit_addition!(pc_plus_pc, pc, pc, "2");
test_unit_addition!(pc_plus_mm, pc, mm, "1.2362204724");
test_unit_addition!(pc_plus_q, pc, q, "1.0590551181");
test_unit_addition!(pc_plus_pt, pc, pt, "1.0833333333");
test_unit_addition!(pc_plus_px, pc, px, "1.0625");

test_unit_addition!(mm_plus_in, mm, in, "26.4");
test_unit_addition!(mm_plus_cm, mm, cm, "11");
test_unit_addition!(mm_plus_pc, mm, pc, "5.2333333333");
test_unit_addition!(mm_plus_mm, mm, mm, "2");
test_unit_addition!(mm_plus_q, mm, q, "1.25");
test_unit_addition!(mm_plus_pt, mm, pt, "1.3527777778");
test_unit_addition!(mm_plus_px, mm, px, "1.2645833333");

test_unit_addition!(q_plus_in, q, in, "102.6");
test_unit_addition!(q_plus_cm, q, cm, "41");
test_unit_addition!(q_plus_pc, q, pc, "17.9333333333");
test_unit_addition!(q_plus_mm, q, mm, "5");
test_unit_addition!(q_plus_q, q, q, "2");
test_unit_addition!(q_plus_pt, q, pt, "2.4111111111");
test_unit_addition!(q_plus_px, q, px, "2.0583333333");

test_unit_addition!(pt_plus_in, pt, in, "73");
test_unit_addition!(pt_plus_cm, pt, cm, "29.3464566929");
test_unit_addition!(pt_plus_pc, pt, pc, "13");
test_unit_addition!(pt_plus_mm, pt, mm, "3.8346456693");
test_unit_addition!(pt_plus_q, pt, q, "1.7086614173");
test_unit_addition!(pt_plus_pt, pt, pt, "2");
test_unit_addition!(pt_plus_px, pt, px, "1.75");

test_unit_addition!(px_plus_in, px, in, "97");
test_unit_addition!(px_plus_cm, px, cm, "38.7952755906");
test_unit_addition!(px_plus_pc, px, pc, "17");
test_unit_addition!(px_plus_mm, px, mm, "4.7795275591");
test_unit_addition!(px_plus_q, px, q, "1.9448818898");
test_unit_addition!(px_plus_pt, px, pt, "2.3333333333");
test_unit_addition!(px_plus_px, px, px, "2");

test_unit_addition!(deg_plus_deg, deg, deg, "2");
test_unit_addition!(deg_plus_grad, deg, grad, "1.9");
test_unit_addition!(deg_plus_rad, deg, rad, "58.2957795131");
test_unit_addition!(deg_plus_turn, deg, turn, "361");

test_unit_addition!(grad_plus_deg, grad, deg, "2.1111111111");
test_unit_addition!(grad_plus_grad, grad, grad, "2");
test_unit_addition!(grad_plus_rad, grad, rad, "64.6619772368");
test_unit_addition!(grad_plus_turn, grad, turn, "401");

test_unit_addition!(rad_plus_deg, rad, deg, "1.0174532925");
test_unit_addition!(rad_plus_grad, rad, grad, "1.0157079633");
test_unit_addition!(rad_plus_rad, rad, rad, "2");
test_unit_addition!(rad_plus_turn, rad, turn, "7.2831853072");

test_unit_addition!(turn_plus_deg, turn, deg, "1.0027777778");
test_unit_addition!(turn_plus_grad, turn, grad, "1.0025");
test_unit_addition!(turn_plus_rad, turn, rad, "1.1591549431");
test_unit_addition!(turn_plus_turn, turn, turn, "2");

test_unit_addition!(s_plus_s, s, s, "2");
test_unit_addition!(s_plus_ms, s, ms, "1.001");

test_unit_addition!(ms_plus_s, ms, s, "1001");
test_unit_addition!(ms_plus_ms, ms, ms, "2");

test_unit_addition!(Hz_plus_Hz, Hz, Hz, "2");
test_unit_addition!(Hz_plus_kHz, Hz, kHz, "1001");

test_unit_addition!(kHz_plus_Hz, kHz, Hz, "1.001");
test_unit_addition!(kHz_plus_kHz, kHz, kHz, "2");

test_unit_addition!(dpi_plus_dpi, dpi, dpi, "2");
test_unit_addition!(dpi_plus_dpcm, dpi, dpcm, "3.54");
test_unit_addition!(dpi_plus_dppx, dpi, dppx, "97");

test_unit_addition!(dpcm_plus_dpi, dpcm, dpi, "1.3937007874");
test_unit_addition!(dpcm_plus_dpcm, dpcm, dpcm, "2");
test_unit_addition!(dpcm_plus_dppx, dpcm, dppx, "38.7952755906");

test_unit_addition!(dppx_plus_dpi, dppx, dpi, "1.0104166667");
test_unit_addition!(dppx_plus_dpcm, dppx, dpcm, "1.0264583333");
test_unit_addition!(dppx_plus_dppx, dppx, dppx, "2");
