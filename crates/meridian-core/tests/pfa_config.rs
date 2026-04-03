use meridian_core::render::{
    PFA_BLUE_TOP_BAR_COLOR, PFA_GREEN_TOP_BAR_COLOR, PFA_RED_TOP_BAR_COLOR,
    PfaKeyboardProjectorConfig, PfaNoteProjectorConfig,
};

#[test]
fn pfa_note_defaults_match_zenith_master() {
    let config = PfaNoteProjectorConfig::default();
    assert!(!config.same_width_notes);
    assert_eq!(config.border_width, 1.0);
}

#[test]
fn pfa_keyboard_defaults_match_zenith_master() {
    let config = PfaKeyboardProjectorConfig::default();
    assert!(!config.same_width_notes);
    assert!(!config.middle_c);
    assert_eq!(config.top_bar_color, PFA_RED_TOP_BAR_COLOR);
}

#[test]
fn pfa_top_bar_uses_hex_color() {
    let config = PfaKeyboardProjectorConfig {
        top_bar_color: "#CC331A".into(),
        ..PfaKeyboardProjectorConfig::default()
    };
    let expected = [204.0 / 255.0, 51.0 / 255.0, 26.0 / 255.0];
    assert_eq!(config.resolved_top_bar_rgb(), expected);
    assert_eq!(
        config.resolved_top_bar_gradient(),
        (
            [expected[0] * 0.5, expected[1] * 0.5, expected[2] * 0.5, 1.0],
            [expected[0], expected[1], expected[2], 1.0]
        )
    );
}

#[test]
fn pfa_preset_names_are_detected_from_hex_colors() {
    let blue = PfaKeyboardProjectorConfig {
        top_bar_color: PFA_BLUE_TOP_BAR_COLOR.into(),
        ..PfaKeyboardProjectorConfig::default()
    };
    let green = PfaKeyboardProjectorConfig {
        top_bar_color: PFA_GREEN_TOP_BAR_COLOR.into(),
        ..PfaKeyboardProjectorConfig::default()
    };

    assert_eq!(blue.top_bar_preset_name(), Some("blue"));
    assert_eq!(green.top_bar_preset_name(), Some("green"));
}

#[test]
fn setting_top_bar_color_normalizes_hex_strings() {
    let mut config = PfaKeyboardProjectorConfig::default();
    assert!(config.set_top_bar_color("3366cc"));
    assert_eq!(config.top_bar_color, "#3366CC");
    assert_eq!(config.top_bar_preset_name(), None);
    assert_eq!(
        config.resolved_top_bar_rgb(),
        [51.0 / 255.0, 102.0 / 255.0, 204.0 / 255.0]
    );
}
