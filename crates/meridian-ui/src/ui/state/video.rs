use std::sync::{Arc, Mutex};

use super::super::view::App;
use super::super::view_model::UiViewModel;
use super::formatting::{file_name_or_full, format_view_range_numeric};
use meridian_core::protocol::{StateSnapshot, VideoRenderStatus};
use meridian_core::render::{
    KeyboardHeightSpec, KeyboardProjectorConfig, NotePaletteConfig, NoteProjectorConfig,
    PFA_RED_TOP_BAR_COLOR, ProjectorBackgroundConfig, ProjectorBackgroundScalingMode,
    ProjectorImageConfig, SceneConfig, TextAlignment, TextAnchor, TextOverlayConfig, TextRowConfig,
    TextSceneConfig, TextStyleConfig, TextValueFormat, TextValueSource, ThreeDSceneConfig,
    ZenithPaletteSpec,
};

pub(super) fn scene_summary(scene: &SceneConfig) -> String {
    match scene {
        SceneConfig::TwoD(scene) => format!(
            "2D / {} notes / {} keyboard / {}",
            note_name(&scene.notes),
            keyboard_name(&scene.keyboard),
            height_name(&scene.keyboard_height),
        ),
        SceneConfig::ThreeD(ThreeDSceneConfig::PianoTrailClassic(_)) => {
            "3D / Piano Trail Classic".into()
        }
        SceneConfig::Text(config) => {
            format!(
                "Text / {} overlays / {} styles",
                config.overlays.len(),
                config.styles.len()
            )
        }
    }
}

pub(super) fn note_name(config: &NoteProjectorConfig) -> &'static str {
    match config {
        NoteProjectorConfig::Flat(_) => "Flat",
        NoteProjectorConfig::Pfa(_) => "PFA",
    }
}

pub(super) fn keyboard_name(config: &KeyboardProjectorConfig) -> &'static str {
    match config {
        KeyboardProjectorConfig::Flat(_) => "Flat",
        KeyboardProjectorConfig::Pfa(_) => "PFA",
    }
}

pub(super) fn height_name(config: &KeyboardHeightSpec) -> &'static str {
    match config {
        KeyboardHeightSpec::ScreenPercent { .. } => "screen %",
        KeyboardHeightSpec::AspectRatio { .. } => "aspect",
    }
}

pub(super) fn renderer_summary(scene: &SceneConfig) -> &'static str {
    match scene {
        SceneConfig::TwoD(scene) => match (&scene.notes, &scene.keyboard) {
            (NoteProjectorConfig::Pfa(_), KeyboardProjectorConfig::Pfa(_)) => "pfa",
            (NoteProjectorConfig::Flat(_), KeyboardProjectorConfig::Flat(_)) => "flat",
            _ => "mixed",
        },
        SceneConfig::ThreeD(_) => "3d",
        SceneConfig::Text(_) => "text",
    }
}

pub(super) fn apply_video_scene_to_app(
    app: &App,
    state: &StateSnapshot,
    shared_state: &Arc<Mutex<UiViewModel>>,
) {
    app.set_video_view_range_value_text(format_view_range_numeric(state.view_range).into());
    app.set_video_view_range_value_float(state.view_range as f32);
    app.set_video_first_key_text(state.first_key.to_string().into());
    app.set_video_last_key_text(state.last_key.to_string().into());
    apply_background_to_app(app, state.scene.background());

    match &state.scene {
        SceneConfig::TwoD(config) => {
            match config.keyboard_height {
                KeyboardHeightSpec::ScreenPercent { height } => {
                    app.set_video_keyboard_height_mode_text("screen_percent".into());
                    app.set_video_keyboard_height_value_text(format!("{height:.5}").into());
                    app.set_video_keyboard_height_value_float(height);
                }
                KeyboardHeightSpec::AspectRatio { ratio } => {
                    app.set_video_keyboard_height_mode_text("aspect_ratio".into());
                    app.set_video_keyboard_height_value_text(format!("{ratio:.5}").into());
                    app.set_video_keyboard_height_value_float(ratio);
                }
            }

            match &config.notes {
                NoteProjectorConfig::Flat(notes) => {
                    app.set_video_note_same_width_text("off".into());
                    app.set_video_border_width_text("1.0".into());
                    app.set_video_border_width_float(1.0);
                    apply_palette_to_app(app, &notes.palette);
                }
                NoteProjectorConfig::Pfa(notes) => {
                    app.set_video_note_same_width_text(on_off(notes.same_width_notes).into());
                    app.set_video_border_width_text(format!("{:.1}", notes.border_width).into());
                    app.set_video_border_width_float(notes.border_width);
                    apply_palette_to_app(app, &notes.palette);
                }
            }

            match &config.keyboard {
                KeyboardProjectorConfig::Flat(_) => {
                    app.set_video_keyboard_same_width_text("off".into());
                    app.set_video_middle_c_text("off".into());
                    app.set_video_top_color_text("red".into());
                    app.set_video_top_bar_color_text(PFA_RED_TOP_BAR_COLOR.into());
                }
                KeyboardProjectorConfig::Pfa(keyboard) => {
                    app.set_video_keyboard_same_width_text(
                        on_off(keyboard.same_width_notes).into(),
                    );
                    app.set_video_middle_c_text(on_off(keyboard.middle_c).into());
                    app.set_video_top_color_text(
                        keyboard.top_bar_preset_name().unwrap_or("custom").into(),
                    );
                    app.set_video_top_bar_color_text(keyboard.top_bar_color.clone().into());
                }
            }

            apply_ptc_defaults_to_app(app);
        }
        SceneConfig::ThreeD(ThreeDSceneConfig::PianoTrailClassic(config)) => {
            app.set_video_keyboard_height_mode_text("aspect_ratio".into());
            app.set_video_keyboard_height_value_text("0.08494".into());
            app.set_video_keyboard_height_value_float(0.08494);
            app.set_video_note_same_width_text("off".into());
            app.set_video_border_width_text("1.0".into());
            app.set_video_border_width_float(1.0);
            app.set_video_keyboard_same_width_text("off".into());
            app.set_video_middle_c_text("off".into());
            app.set_video_top_color_text("red".into());
            app.set_video_top_bar_color_text(PFA_RED_TOP_BAR_COLOR.into());
            apply_palette_to_app(app, &config.palette);

            app.set_video_ptc_same_width_text(on_off(config.same_width_notes).into());
            app.set_video_ptc_fov_text(format!("{:.1}", config.fov.to_degrees()).into());
            app.set_video_ptc_fov_float(config.fov.to_degrees());
            app.set_video_ptc_view_height_text(format!("{:.2}", config.view_height).into());
            app.set_video_ptc_view_height_float(config.view_height);
            app.set_video_ptc_view_offset_text(format!("{:.2}", config.view_offset).into());
            app.set_video_ptc_view_offset_float(config.view_offset);
            app.set_video_ptc_view_pan_text(format!("{:.2}", config.view_pan).into());
            app.set_video_ptc_view_pan_float(config.view_pan);
            app.set_video_ptc_cam_ang_text(format!("{:.2}", config.cam_ang.to_degrees()).into());
            app.set_video_ptc_cam_ang_float(config.cam_ang.to_degrees());
            app.set_video_ptc_cam_rot_text(format!("{:.2}", config.cam_rot.to_degrees()).into());
            app.set_video_ptc_cam_rot_float(config.cam_rot.to_degrees());
            app.set_video_ptc_cam_spin_text(format!("{:.2}", config.cam_spin.to_degrees()).into());
            app.set_video_ptc_cam_spin_float(config.cam_spin.to_degrees());
            app.set_video_ptc_viewdist_text(format!("{:.1}", config.viewdist).into());
            app.set_video_ptc_viewdist_float(config.viewdist);
            app.set_video_ptc_viewback_text(format!("{:.2}", config.viewback).into());
            app.set_video_ptc_viewback_float(config.viewback);
            app.set_video_ptc_vertical_notes_text(on_off(config.vertical_notes).into());
            app.set_video_ptc_note_down_speed_text(format!("{:.2}", config.note_down_speed).into());
            app.set_video_ptc_note_down_speed_float(config.note_down_speed);
            app.set_video_ptc_note_up_speed_text(format!("{:.2}", config.note_up_speed).into());
            app.set_video_ptc_note_up_speed_float(config.note_up_speed);
            app.set_video_ptc_box_notes_text(on_off(config.box_notes).into());
            app.set_video_ptc_light_shade_text(on_off(config.light_shade).into());
            app.set_video_ptc_show_keyboard_text(on_off(config.show_keyboard).into());
            app.set_video_ptc_tilt_keys_text(on_off(config.tilt_keys).into());
            app.set_video_ptc_eat_notes_text(on_off(config.eat_notes).into());
            app.set_video_ptc_aura_enabled_text(on_off(config.aura_enabled).into());
            app.set_video_ptc_aura_strength_text(format!("{:.1}", config.aura_strength).into());
            app.set_video_ptc_aura_strength_float(config.aura_strength);
            app.set_video_ptc_notes_change_size_text(on_off(config.notes_change_size).into());
            app.set_video_ptc_notes_change_tint_text(on_off(config.notes_change_tint).into());
            app.set_video_ptc_use_vel_text(on_off(config.use_vel).into());
            match &config.aura_image {
                ProjectorImageConfig::Builtin { name } => {
                    app.set_video_ptc_aura_image_source_text("builtin".into());
                    app.set_video_ptc_aura_image_name_text(name.clone().into());
                    app.set_video_ptc_aura_image_path_text("(aura png)".into());
                }
                ProjectorImageConfig::PngFile { path } => {
                    app.set_video_ptc_aura_image_source_text("png_file".into());
                    app.set_video_ptc_aura_image_name_text("ring".into());
                    app.set_video_ptc_aura_image_path_text(path.clone().into());
                }
            }
        }
        SceneConfig::Text(config) => {
            app.set_video_keyboard_height_mode_text("aspect_ratio".into());
            app.set_video_keyboard_height_value_text("0.08494".into());
            app.set_video_keyboard_height_value_float(0.08494);
            app.set_video_note_same_width_text("off".into());
            app.set_video_border_width_text("1.0".into());
            app.set_video_border_width_float(1.0);
            app.set_video_keyboard_same_width_text("off".into());
            app.set_video_middle_c_text("off".into());
            app.set_video_top_color_text("red".into());
            app.set_video_top_bar_color_text(PFA_RED_TOP_BAR_COLOR.into());
            app.set_video_palette_source_text("default_track_colors".into());
            app.set_video_palette_kind_text("random".into());
            app.set_video_palette_randomize_text("off".into());
            apply_ptc_defaults_to_app(app);
            apply_text_scene_to_app(app, config, shared_state);
        }
    }
}

fn apply_text_scene_to_app(
    app: &App,
    config: &TextSceneConfig,
    shared_state: &Arc<Mutex<UiViewModel>>,
) {
    let _ = shared_state;

    let overlay = simple_text_overlay(config);
    let style = simple_text_style(config, &overlay);
    let template_text = simple_text_template(config);

    app.set_video_text_background_color_text(config.background_color.clone().into());
    app.set_video_text_overlay_anchor_text(text_anchor_key(overlay.anchor).into());
    app.set_video_text_overlay_alignment_text(text_alignment_key(overlay.alignment).into());
    app.set_video_text_overlay_background_color_text(
        overlay.background_color.clone().unwrap_or_default().into(),
    );
    app.set_video_text_overlay_x_float(overlay.x);
    app.set_video_text_overlay_y_float(overlay.y);
    app.set_video_text_overlay_width_float(overlay.width);
    app.set_video_text_row_text_text(template_text.into());
    app.set_video_text_style_font_family_text(style.font_family.clone().into());
    app.set_video_text_style_color_text(style.color.clone().into());
    app.set_video_text_style_font_size_float(style.font_size as f32);
    app.set_video_text_style_line_spacing_float(style.line_spacing);
}

fn simple_text_overlay(config: &TextSceneConfig) -> TextOverlayConfig {
    config.overlays.first().cloned().unwrap_or_default()
}

fn simple_text_style(config: &TextSceneConfig, overlay: &TextOverlayConfig) -> TextStyleConfig {
    config
        .style_named(overlay.resolved_style_name())
        .or_else(|| config.style_named(config.default_style.as_str()))
        .or_else(|| config.style_named("body"))
        .or_else(|| config.styles.first())
        .cloned()
        .unwrap_or_default()
}

fn simple_text_template(config: &TextSceneConfig) -> String {
    config
        .overlays
        .iter()
        .map(|overlay| {
            overlay
                .rows
                .iter()
                .map(row_to_template_text)
                .filter(|row| !row.trim().is_empty())
                .collect::<Vec<_>>()
                .join("\n")
        })
        .filter(|overlay| !overlay.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn row_to_template_text(row: &TextRowConfig) -> String {
    match row {
        TextRowConfig::PlainText { text, .. } => text.clone(),
        TextRowConfig::Metric {
            label,
            source,
            format,
            prefix,
            suffix,
            ..
        } => {
            let mut out = String::new();
            if !label.trim().is_empty() {
                out.push_str(label.trim());
                out.push_str(": ");
            }
            if !prefix.is_empty() {
                out.push_str(prefix);
            }
            out.push_str(&template_token(*source, Some(*format)));
            if !suffix.is_empty() {
                out.push_str(suffix);
            }
            out
        }
        TextRowConfig::MetricPair {
            label,
            primary_source,
            primary_format,
            secondary_source,
            secondary_format,
            separator,
            ..
        } => {
            let mut out = String::new();
            if !label.trim().is_empty() {
                out.push_str(label.trim());
                out.push_str(": ");
            }
            out.push_str(&template_token(*primary_source, Some(*primary_format)));
            out.push_str(separator);
            out.push_str(&template_token(
                *secondary_source,
                secondary_format.or(Some(*primary_format)),
            ));
            out
        }
    }
}

fn template_token(source: TextValueSource, format: Option<TextValueFormat>) -> String {
    let source = match source {
        TextValueSource::MidiName => "midi.name",
        TextValueSource::RendererName => "renderer.name",
        TextValueSource::ViewportWidth => "viewport.width",
        TextValueSource::ViewportHeight => "viewport.height",
        TextValueSource::CurrentTimeSeconds => "time.current",
        TextValueSource::RemainingTimeSeconds => "time.remaining",
        TextValueSource::MidiLengthSeconds => "time.length",
        TextValueSource::CurrentTick => "tick.current",
        TextValueSource::RemainingTick => "tick.remaining",
        TextValueSource::MidiLengthTick => "tick.length",
        TextValueSource::TotalNotes => "notes.total",
        TextValueSource::PassedNotes => "notes.passed",
        TextValueSource::RemainingNotes => "notes.remaining",
        TextValueSource::VisibleNotes => "notes.visible",
        TextValueSource::ActiveKeys => "keys.active",
        TextValueSource::CurrentPolyphony => "polyphony.current",
        TextValueSource::CurrentBpm => "tempo.bpm",
        TextValueSource::CurrentNps1s => "density.nps1",
        TextValueSource::CurrentNps2s => "density.nps2",
    };
    match format {
        Some(format) => format!("{{{{{source}|{}}}}}", template_format_name(format)),
        None => format!("{{{{{source}}}}}"),
    }
}

fn template_format_name(format: TextValueFormat) -> &'static str {
    match format {
        TextValueFormat::Raw => "raw",
        TextValueFormat::Integer => "int",
        TextValueFormat::Decimal1 => "0.0",
        TextValueFormat::Decimal2 => "0.00",
        TextValueFormat::Decimal3 => "0.000",
        TextValueFormat::Clock => "clock",
        TextValueFormat::Seconds1 => "s1",
        TextValueFormat::Seconds2 => "s2",
        TextValueFormat::Bpm => "bpm",
        TextValueFormat::Ticks => "ticks",
    }
}

pub(super) fn text_anchor_key(anchor: TextAnchor) -> &'static str {
    match anchor {
        TextAnchor::TopLeft => "top_left",
        TextAnchor::TopRight => "top_right",
        TextAnchor::BottomLeft => "bottom_left",
        TextAnchor::BottomRight => "bottom_right",
        TextAnchor::Center => "center",
    }
}

pub(super) fn text_alignment_key(alignment: TextAlignment) -> &'static str {
    match alignment {
        TextAlignment::Left => "left",
        TextAlignment::Center => "center",
        TextAlignment::Right => "right",
    }
}

pub(super) fn apply_ptc_defaults_to_app(app: &App) {
    app.set_video_ptc_same_width_text("on".into());
    app.set_video_ptc_fov_text("60.0".into());
    app.set_video_ptc_fov_float(60.0);
    app.set_video_ptc_view_height_text("0.50".into());
    app.set_video_ptc_view_height_float(0.50);
    app.set_video_ptc_view_offset_text("0.40".into());
    app.set_video_ptc_view_offset_float(0.40);
    app.set_video_ptc_view_pan_text("0.00".into());
    app.set_video_ptc_view_pan_float(0.0);
    app.set_video_ptc_cam_ang_text(format!("{:.2}", 0.56_f32.to_degrees()).into());
    app.set_video_ptc_cam_ang_float(0.56_f32.to_degrees());
    app.set_video_ptc_cam_rot_text("0.00".into());
    app.set_video_ptc_cam_rot_float(0.0);
    app.set_video_ptc_cam_spin_text("0.00".into());
    app.set_video_ptc_cam_spin_float(0.0);
    app.set_video_ptc_viewdist_text("14.0".into());
    app.set_video_ptc_viewdist_float(14.0);
    app.set_video_ptc_viewback_text("0.20".into());
    app.set_video_ptc_viewback_float(0.20);
    app.set_video_ptc_vertical_notes_text("off".into());
    app.set_video_ptc_note_down_speed_text("0.60".into());
    app.set_video_ptc_note_down_speed_float(0.60);
    app.set_video_ptc_note_up_speed_text("0.20".into());
    app.set_video_ptc_note_up_speed_float(0.20);
    app.set_video_ptc_box_notes_text("on".into());
    app.set_video_ptc_light_shade_text("off".into());
    app.set_video_ptc_show_keyboard_text("on".into());
    app.set_video_ptc_tilt_keys_text("on".into());
    app.set_video_ptc_eat_notes_text("off".into());
    app.set_video_ptc_aura_enabled_text("on".into());
    app.set_video_ptc_aura_strength_text("2.0".into());
    app.set_video_ptc_aura_strength_float(2.0);
    app.set_video_ptc_notes_change_size_text("off".into());
    app.set_video_ptc_notes_change_tint_text("off".into());
    app.set_video_ptc_use_vel_text("off".into());
    app.set_video_ptc_aura_image_source_text("builtin".into());
    app.set_video_ptc_aura_image_name_text("ring".into());
    app.set_video_ptc_aura_image_path_text("(aura png)".into());
}

pub(super) fn apply_background_to_app(app: &App, background: &ProjectorBackgroundConfig) {
    match background {
        ProjectorBackgroundConfig::None => {
            app.set_video_background_source_text("none".into());
            app.set_video_background_scale_text("stretch".into());
            app.set_video_background_path_text("(background png)".into());
        }
        ProjectorBackgroundConfig::PngFile { path, scaling } => {
            app.set_video_background_source_text("png_file".into());
            app.set_video_background_scale_text(match scaling {
                ProjectorBackgroundScalingMode::Stretch => "stretch".into(),
                ProjectorBackgroundScalingMode::Cover => "cover".into(),
            });
            app.set_video_background_path_text(path.clone().into());
        }
    }
}

pub(super) fn apply_video_render_status_to_app(app: &App, status: &VideoRenderStatus) {
    match status {
        VideoRenderStatus::Idle => {
            app.set_video_render_progress(0.0);
            app.set_video_render_status("Idle".into());
            app.set_video_render_elapsed("—".into());
            app.set_video_render_output_text(
                format!("output.{}", app.get_render_video_container_text()).into(),
            );
        }
        VideoRenderStatus::Running {
            output,
            fps,
            total_frames,
            frame_index,
            audio_progress,
            ..
        } => {
            let progress = if *total_frames == 0 {
                0.0
            } else {
                *frame_index as f32 / *total_frames as f32
            };
            let total_duration_seconds = if *fps > 0.0 {
                *total_frames as f64 / *fps
            } else {
                0.0
            };
            let rendered_seconds = if *fps > 0.0 {
                *frame_index as f64 / *fps
            } else {
                0.0
            };
            app.set_video_render_progress(progress);
            app.set_video_render_status("Rendering video".into());
            app.set_video_render_elapsed(format!("{rendered_seconds:.1} s").into());
            app.set_video_render_output_text(file_name_or_full(output).into());
            if let Some(audio_progress) = audio_progress {
                let progress = if total_duration_seconds <= 0.0 {
                    0.0
                } else {
                    (audio_progress.rendered_seconds / total_duration_seconds).clamp(0.0, 1.0)
                        as f32
                };
                let status = if audio_progress.event_index >= audio_progress.total_events
                    && audio_progress.total_events > 0
                {
                    "Audio ready"
                } else {
                    "Rendering audio"
                };
                app.set_audio_render_progress(progress);
                app.set_audio_render_status(status.into());
                app.set_audio_render_elapsed(
                    format!("{:.1} s", audio_progress.rendered_seconds).into(),
                );
                app.set_audio_render_output_text(file_name_or_full(output).into());
            }
        }
        VideoRenderStatus::Cancelling {
            output,
            fps,
            total_frames,
            frame_index,
            audio_progress,
            ..
        } => {
            let progress = if *total_frames == 0 {
                0.0
            } else {
                *frame_index as f32 / *total_frames as f32
            };
            let total_duration_seconds = if *fps > 0.0 {
                *total_frames as f64 / *fps
            } else {
                0.0
            };
            let rendered_seconds = if *fps > 0.0 {
                *frame_index as f64 / *fps
            } else {
                0.0
            };
            app.set_video_render_progress(progress);
            app.set_video_render_status("Cancelling render".into());
            app.set_video_render_elapsed(format!("{rendered_seconds:.1} s").into());
            app.set_video_render_output_text(file_name_or_full(output).into());
            if let Some(audio_progress) = audio_progress {
                let progress = if total_duration_seconds <= 0.0 {
                    0.0
                } else {
                    (audio_progress.rendered_seconds / total_duration_seconds).clamp(0.0, 1.0)
                        as f32
                };
                app.set_audio_render_progress(progress);
                app.set_audio_render_status("Cancelling audio".into());
                app.set_audio_render_elapsed(
                    format!("{:.1} s", audio_progress.rendered_seconds).into(),
                );
                app.set_audio_render_output_text(file_name_or_full(output).into());
            }
        }
    }
}

pub(super) fn apply_palette_to_app(app: &App, palette: &NotePaletteConfig) {
    match palette {
        NotePaletteConfig::DefaultTrackColors => {
            app.set_video_palette_source_text("default_track_colors".into());
            app.set_video_palette_kind_text("random".into());
            app.set_video_palette_randomize_text("on".into());
            app.set_video_palette_path_text("(palette png)".into());
        }
        NotePaletteConfig::ZenithPalette { palette, randomize } => {
            app.set_video_palette_source_text("zenith_palette".into());
            app.set_video_palette_randomize_text(on_off(*randomize).into());
            match palette {
                ZenithPaletteSpec::Random => {
                    app.set_video_palette_kind_text("random".into());
                    app.set_video_palette_path_text("(palette png)".into());
                }
                ZenithPaletteSpec::RandomGradients => {
                    app.set_video_palette_kind_text("random_gradients".into());
                    app.set_video_palette_path_text("(palette png)".into());
                }
                ZenithPaletteSpec::PngFile { path } => {
                    app.set_video_palette_kind_text("png_file".into());
                    app.set_video_palette_path_text(path.display().to_string().into());
                }
            }
        }
    }
}

pub(super) fn on_off(value: bool) -> &'static str {
    if value { "on" } else { "off" }
}
