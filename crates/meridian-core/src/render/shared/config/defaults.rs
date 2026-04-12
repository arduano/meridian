use super::*;

pub(super) const fn default_border_width() -> f32 {
    1.0
}

pub(super) fn default_top_bar_color() -> String {
    PFA_RED_TOP_BAR_COLOR.to_string()
}

pub(super) const fn default_top_bar_rgb() -> [f32; 3] {
    [149.0 / 255.0, 10.0 / 255.0, 6.0 / 255.0]
}

pub(super) const fn default_piano_trail_classic_same_width_notes() -> bool {
    true
}

pub(super) const fn default_piano_trail_classic_fov() -> f32 {
    std::f32::consts::PI / 3.0
}

pub(super) const fn default_piano_trail_classic_view_height() -> f32 {
    0.5
}

pub(super) const fn default_piano_trail_classic_view_offset() -> f32 {
    0.4
}

pub(super) const fn default_piano_trail_classic_cam_ang() -> f32 {
    0.56
}

pub(super) const fn default_piano_trail_classic_viewdist() -> f32 {
    14.0
}

pub(super) const fn default_piano_trail_classic_viewback() -> f32 {
    0.2
}

pub(super) const fn default_piano_trail_classic_note_down_speed() -> f32 {
    0.6
}

pub(super) const fn default_piano_trail_classic_note_up_speed() -> f32 {
    0.2
}

pub(super) const fn default_piano_trail_classic_show_keyboard() -> bool {
    true
}

pub(super) const fn default_piano_trail_classic_tilt_keys() -> bool {
    true
}

pub(super) const fn default_piano_trail_classic_aura_strength() -> f32 {
    2.0
}

pub(super) const fn default_piano_trail_classic_aura_enabled() -> bool {
    true
}

pub(super) const fn default_piano_trail_classic_notes_change_tint() -> bool {
    true
}

pub(super) fn default_piano_trail_classic_aura_image() -> ProjectorImageConfig {
    ProjectorImageConfig::Builtin {
        name: "ring".to_string(),
    }
}

pub(super) fn default_text_background_color() -> String {
    "#102B1E".to_string()
}

pub(super) fn default_text_default_style_name() -> String {
    "body".to_string()
}

pub(super) fn default_text_color() -> String {
    "#D7FBE8".to_string()
}

pub(super) fn default_text_font_family() -> String {
    "sans-serif".to_string()
}

pub(super) fn default_text_style_name() -> String {
    "body".to_string()
}

pub(super) const fn default_text_pixel_scale() -> u32 {
    48
}

pub(super) const fn default_text_x() -> f32 {
    0.08
}

pub(super) const fn default_text_y() -> f32 {
    0.10
}

pub(super) const fn default_text_line_spacing() -> f32 {
    10.0
}

pub(super) const fn default_text_overlay_width() -> f32 {
    0.42
}

pub(super) const fn default_text_overlay_padding() -> f32 {
    0.0
}

pub(super) const fn default_text_overlay_row_gap() -> f32 {
    6.0
}
