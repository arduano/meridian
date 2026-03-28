use meridian_core::render::{PfaKeyboardProjectorConfig, PfaNoteProjectorConfig, PfaTopColor};

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
    assert_eq!(config.top_color, PfaTopColor::Red);
    assert_eq!(config.top_bar_rgb, [0.585, 0.0392, 0.0249]);
}

#[test]
fn pfa_red_top_bar_uses_custom_rgb() {
    let config = PfaKeyboardProjectorConfig {
        top_color: PfaTopColor::Red,
        top_bar_rgb: [0.8, 0.2, 0.1],
        ..PfaKeyboardProjectorConfig::default()
    };
    assert_eq!(config.resolved_top_bar_rgb(), [0.8, 0.2, 0.1]);
    assert_eq!(
        config.resolved_top_bar_gradient(),
        ([0.4, 0.1, 0.05, 1.0], [0.8, 0.2, 0.1, 1.0])
    );
}

#[test]
fn pfa_blue_and_green_use_zenith_presets() {
    let blue = PfaKeyboardProjectorConfig {
        top_color: PfaTopColor::Blue,
        top_bar_rgb: [1.0, 0.0, 0.0],
        ..PfaKeyboardProjectorConfig::default()
    };
    let green = PfaKeyboardProjectorConfig {
        top_color: PfaTopColor::Green,
        top_bar_rgb: [1.0, 0.0, 0.0],
        ..PfaKeyboardProjectorConfig::default()
    };

    assert_eq!(blue.resolved_top_bar_rgb(), [0.0392, 0.0249, 0.585]);
    assert_eq!(green.resolved_top_bar_rgb(), [0.0249, 0.585, 0.0392]);
}

#[test]
fn setting_custom_top_bar_rgb_switches_back_to_red_mode() {
    let mut config = PfaKeyboardProjectorConfig {
        top_color: PfaTopColor::Blue,
        ..PfaKeyboardProjectorConfig::default()
    };
    config.set_top_bar_rgb([0.2, 0.3, 0.4]);

    assert_eq!(config.top_color, PfaTopColor::Red);
    assert_eq!(config.top_bar_rgb, [0.2, 0.3, 0.4]);
    assert_eq!(config.resolved_top_bar_rgb(), [0.2, 0.3, 0.4]);
}
