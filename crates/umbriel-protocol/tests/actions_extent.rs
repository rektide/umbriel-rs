use umbriel_protocol::{
    REQUEST_LIMIT,
    action::{Action, ExtentDelta, ExtentFraction, ValidationError, catalog},
    codec,
};

#[derive(serde::Deserialize)]
struct Msg {
    cmd: String,
    arg: String,
}

// A model of the plain-decimal subset the Rust formatter emits: an optional
// '-', at least one integral digit, at most one interior '.', and fractional
// digits only after it — never exponent notation, a bare '.', or inf/nan
// words. This is deliberately NOT a model of std::from_chars: the native
// general-format branch (keybind_parse.cpp:495/516, observed with a local
// charconv probe) additionally admits '.5', '1.', and scientific spellings
// such as '1e-1' or '5e-324', and parses 'inf' and 'nan' words that the
// upstream finiteness gate then rejects. A leading '+' is rejected by
// from_chars itself; the delta branch strips exactly one '+' first. Because
// our builder output always lies inside this subset, checking emissions
// against the subset proves the native branch accepts them.
fn parse_plain_decimal(arg: &str) -> Option<f64> {
    let unsigned = arg.strip_prefix('-').unwrap_or(arg);
    let (integral, fractional) = match unsigned.split_once('.') {
        Some((integral, fractional)) => (integral, fractional),
        None => (unsigned, ""),
    };
    let digits = |text: &str| !text.is_empty() && text.bytes().all(|byte| byte.is_ascii_digit());
    if !digits(integral) || (unsigned.contains('.') && !digits(fractional)) {
        return None;
    }
    arg.parse().ok()
}

// Upstream's post-parse gates, applied to an argument our subset grammar
// already accepted.
fn fraction_gate(value: f64) -> Option<f64> {
    (value.is_finite() && (0.1..=1.0).contains(&value)).then_some(value)
}

fn delta_gate(value: f64) -> Option<f64> {
    (value.is_finite() && value != 0.0 && value.abs() <= 0.9).then_some(value)
}

// The argument half of an emitted action text, reparsed as a plain decimal.
// Emitted text never carries '+', so no sign stripping is needed here.
fn emitted_argument(text: &str) -> Option<f64> {
    parse_plain_decimal(text.split_once(':')?.1)
}

// Spellings the native branch rejects, each verified against the local
// charconv probe: range-gate failures ('0', '0.09', '1.5', '-0.5'), grammar
// failures ('abc', '0.5x', '', '0.90.1'), the from_chars '+' rejection
// ('+0.5', '++0.1' even after the delta branch strips one '+'), and 'nan',
// which from_chars parses but the upstream finiteness gate rejects.
#[test]
fn gates_reject_what_the_native_branch_rejects() {
    // The fraction branch hands the text to from_chars unchanged, so a
    // leading '+' is rejected natively; no stripping happens here.
    for arg in [
        "0", "0.09", "1.5", "-0.5", "abc", "0.5x", "", "nan", "+0.5", "0.90.1",
    ] {
        let value = parse_plain_decimal(arg);
        assert!(
            value.is_none_or(|value| fraction_gate(value).is_none()),
            "fraction evidence accepted {arg:?}"
        );
    }
    // The delta branch strips exactly one leading '+' before from_chars.
    for arg in [
        "0", "1.5", "-1.0", "abc", "", "++0.1", "0.1x", "nan", "inf", "0.90.1",
    ] {
        let value = parse_plain_decimal(arg.strip_prefix('+').unwrap_or(arg));
        assert!(
            value.is_none_or(|value| delta_gate(value).is_none()),
            "delta evidence accepted {arg:?}"
        );
    }
    // The delta branch, unlike fractions, admits one explicit plus.
    assert_eq!(
        parse_plain_decimal("+0.1".strip_prefix('+').unwrap()).and_then(delta_gate),
        Some(0.1)
    );
    // from_chars itself parses 'inf'/'nan' words (probe-observed); the
    // upstream finiteness gate is what rejects those parsed values.
    assert!(fraction_gate(f64::NAN).is_none());
    assert!(delta_gate(f64::INFINITY).is_none() && delta_gate(f64::NAN).is_none());
}

// The subset grammar is intentionally narrower than from_chars: '.5', '1.',
// and scientific notation are native-admitted spellings our formatter never
// produces. The model rejecting them documents that boundary; it does not
// claim the compositor rejects them.
#[test]
fn the_plain_decimal_model_is_deliberately_narrower_than_from_chars() {
    for arg in [".5", "1.", "1e-1", "1E-1", "0.5e1", "5e-324"] {
        assert!(
            parse_plain_decimal(arg).is_none(),
            "model treated {arg:?} as emitted"
        );
    }
}

#[test]
fn fractions_accept_exactly_the_compositor_range() {
    for value in [
        0.1,
        1.0,
        0.5,
        0.55,
        0.123_456_789_012_345_68,
        0.999_999_999_999_999_9,
        0.1 + 0.2,
    ] {
        let fraction = ExtentFraction::new(value).unwrap();
        assert_eq!(fraction.value(), value);
        let set = Action::window_set_primary_extent(fraction);
        assert_eq!(
            emitted_argument(set.as_str()).and_then(fraction_gate),
            Some(value),
            "{} emitted an unacceptable spelling",
            set.as_str()
        );
        let secondary = Action::window_set_secondary_extent(fraction);
        assert!(
            secondary
                .as_str()
                .starts_with("window-set-secondary-extent:")
        );
    }
    for value in [
        0.0,
        -0.0,
        0.05,
        0.099999,
        1.0000001,
        1.5,
        -0.5,
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
    ] {
        assert!(ExtentFraction::new(value).is_err(), "accepted {value}");
    }
    assert_eq!(
        ExtentFraction::new(1.5).unwrap_err(),
        ValidationError::InvalidFraction { value: 1.5 }
    );
    assert!(matches!(
        ExtentFraction::new(f64::NAN),
        Err(ValidationError::InvalidFraction { value }) if value.is_nan()
    ));
}

#[test]
fn deltas_are_finite_nonzero_and_capped_at_nine_tenths() {
    let builders: [fn(ExtentDelta) -> Action<'static>; 6] = [
        Action::window_modify_primary_extent,
        Action::window_modify_secondary_extent,
        Action::window_modify_width_left,
        Action::window_modify_width_right,
        Action::window_modify_height_up,
        Action::window_modify_height_down,
    ];
    for value in [-0.9, 0.9, -0.2, 0.25, 0.001, -0.899_999_999_999_999_9] {
        let delta = ExtentDelta::new(value).unwrap();
        assert_eq!(delta.value(), value);
        for build in builders {
            let action = build(delta);
            let prefix = action
                .as_str()
                .split_once(':')
                .expect("parameterized spelling");
            assert_eq!(prefix.1, delta_action_spelling(value));
        }
    }
    for value in [
        0.0,
        -0.0,
        0.91,
        -0.91,
        1.0,
        -1.0,
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
    ] {
        assert!(ExtentDelta::new(value).is_err(), "accepted {value}");
    }
    assert_eq!(
        ExtentDelta::new(0.91).unwrap_err(),
        ValidationError::InvalidFractionDelta { value: 0.91 }
    );
}

// The shortest round-trip decimal Rust emits for each value; the emitted
// spelling must reparse to the same value inside the plain-decimal subset,
// which the native branch is observed to accept in full.
fn delta_action_spelling(value: f64) -> String {
    let emitted = format!("{value}");
    assert_eq!(
        parse_plain_decimal(&emitted).and_then(delta_gate),
        Some(value),
        "spelling {emitted} lost the value"
    );
    emitted
}

// Representable tiny deltas stay plain positional decimals: Display never
// switches to exponent notation, and the local charconv probe observed the
// native branch fully consuming these long spellings and reproducing the
// identical f64 bits for both f64::MIN_POSITIVE and the smallest subnormal.
#[test]
fn tiny_deltas_roundtrip_as_plain_positional_decimals() {
    for value in [f64::MIN_POSITIVE, f64::from_bits(1)] {
        let delta = ExtentDelta::new(value).unwrap();
        assert_eq!(delta.value(), value);
        let emitted = delta_action_spelling(value);
        assert!(emitted.starts_with("0.0") && !emitted.contains(['e', 'E']));
        assert_eq!(
            Action::window_modify_height_down(delta)
                .as_str()
                .strip_prefix("window-modify-height-down:"),
            Some(emitted.as_str())
        );
        // Well inside the wire budget despite hundreds of digits.
        assert!(emitted.len() < 400);
    }
    // Zero-adjacent but exactly zero (including -0.0) stays rejected.
    assert!(ExtentDelta::new(0.0).is_err());
    assert!(ExtentDelta::new(-0.0).is_err());
}

#[test]
fn whole_values_emit_plain_decimal_spellings() {
    let cases = [
        (
            Action::window_set_primary_extent(ExtentFraction::new(0.5).unwrap()),
            "window-set-primary-extent:0.5",
        ),
        (
            Action::window_set_primary_extent(ExtentFraction::new(1.0).unwrap()),
            "window-set-primary-extent:1",
        ),
        (
            Action::window_set_secondary_extent(ExtentFraction::new(0.1).unwrap()),
            "window-set-secondary-extent:0.1",
        ),
        (
            Action::window_modify_primary_extent(ExtentDelta::new(-0.2).unwrap()),
            "window-modify-primary-extent:-0.2",
        ),
        (
            Action::window_modify_height_up(ExtentDelta::new(0.25).unwrap()),
            "window-modify-height-up:0.25",
        ),
        (
            Action::window_modify_width_left(ExtentDelta::new(-0.9).unwrap()),
            "window-modify-width-left:-0.9",
        ),
    ];
    for (action, expected) in cases {
        assert_eq!(action.as_str(), expected);
    }
}

#[test]
fn extent_specs_carry_wire_names_and_categories() {
    let specs = [
        (
            catalog::WINDOW_SET_PRIMARY_EXTENT,
            "window-set-primary-extent",
        ),
        (
            catalog::WINDOW_SET_SECONDARY_EXTENT,
            "window-set-secondary-extent",
        ),
        (
            catalog::WINDOW_MODIFY_PRIMARY_EXTENT,
            "window-modify-primary-extent",
        ),
        (
            catalog::WINDOW_MODIFY_SECONDARY_EXTENT,
            "window-modify-secondary-extent",
        ),
        (
            catalog::WINDOW_MODIFY_WIDTH_LEFT,
            "window-modify-width-left",
        ),
        (
            catalog::WINDOW_MODIFY_WIDTH_RIGHT,
            "window-modify-width-right",
        ),
        (catalog::WINDOW_MODIFY_HEIGHT_UP, "window-modify-height-up"),
        (
            catalog::WINDOW_MODIFY_HEIGHT_DOWN,
            "window-modify-height-down",
        ),
    ];
    let fraction: &[&str] = &["window-set-primary-extent", "window-set-secondary-extent"];
    for (spec, name) in specs {
        assert_eq!(spec.name, name);
        let expected = if fraction.contains(&name) {
            catalog::Category::Fraction
        } else {
            catalog::Category::FractionDelta
        };
        assert_eq!(spec.category, expected, "{name}");
    }
}

#[test]
fn extent_actions_encode_reuse_text_and_storage() {
    let action = Action::window_modify_width_right(ExtentDelta::new(0.25).unwrap());
    let text_pointer = action.as_str().as_ptr();
    let mut bytes = Vec::with_capacity(128);
    let bytes_pointer = bytes.as_ptr();
    for _ in 0..10 {
        action.request().encode(&mut bytes).unwrap();
        assert_eq!(action.as_str().as_ptr(), text_pointer);
        assert_eq!(bytes.as_ptr(), bytes_pointer);
    }
    let decoded: Msg = codec::decode(&bytes[..bytes.len() - 1]).unwrap();
    assert_eq!(decoded.cmd, "msg");
    assert_eq!(decoded.arg, "window-modify-width-right:0.25");
    assert_eq!(bytes.iter().filter(|&&byte| byte == b'\n').count(), 1);
    // Extent arguments are bounded decimals, so no extent request approaches
    // the size limit; only unbounded-text families need boundary probes.
    assert!(bytes.len() < REQUEST_LIMIT);
}
