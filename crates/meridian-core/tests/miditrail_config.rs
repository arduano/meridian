use meridian_core::render::{
    MiditrailAuraImage, MiditrailSceneConfig, SceneConfig, ThreeDSceneConfig,
};

#[test]
fn miditrail_defaults_match_zenith_master() {
    let config = MiditrailSceneConfig::default();
    assert!(config.same_width_notes);
    assert!((config.fov - std::f32::consts::PI / 3.0).abs() < 1e-6);
    assert!((config.view_height - 0.5).abs() < 1e-6);
    assert!((config.view_offset - 0.4).abs() < 1e-6);
    assert!((config.cam_ang - 0.56).abs() < 1e-6);
    assert!((config.viewdist - 14.0).abs() < 1e-6);
    assert!((config.viewback - 0.2).abs() < 1e-6);
    assert!((config.note_down_speed - 0.6).abs() < 1e-6);
    assert!((config.note_up_speed - 0.2).abs() < 1e-6);
    assert!(config.show_keyboard);
    assert!(config.tilt_keys);
    assert!(config.aura_enabled);
    assert!(config.notes_change_tint);
    assert_eq!(config.aura_image, MiditrailAuraImage::Ring);
}

#[test]
fn three_d_scene_defaults_to_miditrail() {
    let scene = SceneConfig::ThreeD(ThreeDSceneConfig::default());
    match scene {
        SceneConfig::ThreeD(ThreeDSceneConfig::Miditrail(_)) => {}
        _ => panic!("expected miditrail default"),
    }
}
