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

// Independent miniature of the compositor's from_chars branches: one optional
// sign, plain decimal digits, no exponent or trailing garbage. The fraction
// branch rejects '+' outright; the delta branch strips exactly one leading '+'.
fn parse_decimal(arg: &str) -> Option<f64> {
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

fn accepts_fraction(text: &str) -> Option<f64> {
    let arg = text.strip_prefix("window-set-primary-extent:")?;
    let value = parse_decimal(arg)?;
    (0.1..=1.0).contains(&value).then_some(value)
}

fn accepts_delta(text: &str) -> Option<f64> {
    let arg = text.strip_prefix("window-modify-primary-extent:")?;
    let unsigned = arg.strip_prefix('+').unwrap_or(arg);
    let value = parse_decimal(unsigned)?;
    (value != 0.0 && value.abs() <= 0.9).then_some(value)
}

// The upstream conformance spellings the C++ parser rejects.
#[test]
fn the_model_rejects_the_upstream_rejected_spellings() {
    for arg in [
        "0", "0.09", "1.5", "-0.5", "abc", "0.5x", "", "nan", "+0.5", ".5", "1.",
    ] {
        assert!(
            accepts_fraction(&format!("window-set-primary-extent:{arg}")).is_none(),
            "fraction model accepted {arg:?}"
        );
    }
    for arg in [
        "0", "1.5", "-1.0", "abc", "", "++0.1", "0.1x", "nan", "inf", "0.90.1",
    ] {
        assert!(
            accepts_delta(&format!("window-modify-primary-extent:{arg}")).is_none(),
            "delta model accepted {arg:?}"
        );
    }
    // The delta branch, unlike fractions, admits one explicit plus.
    assert_eq!(
        accepts_delta("window-modify-primary-extent:+0.1"),
        Some(0.1)
    );
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
            accepts_fraction(set.as_str()),
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
// spelling must reparse under the same grammar the compositor applies.
fn delta_action_spelling(value: f64) -> String {
    let emitted = format!("{value}");
    assert_eq!(
        accepts_delta(&format!("window-modify-primary-extent:{emitted}")),
        Some(value),
        "spelling {emitted} lost the value"
    );
    emitted
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
