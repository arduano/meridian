use std::{
    env,
    path::Path,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use meridian_core::{
    audio::{AudioBackend, EnvelopeCurveType, ThreadCount},
    protocol::{AudioRenderStatus, CoreEvent, MidiAnalysisData, StateSnapshot},
    render::{
        KeyboardHeightSpec, KeyboardProjectorConfig, NotePaletteConfig, NoteProjectorConfig,
        PfaTopColor, ProjectorImageConfig, RendererKind, SceneConfig, ThreeDSceneConfig,
        ZenithPaletteSpec,
    },
};
use slint::{ModelRc, SharedString, VecModel};

use super::{
    inspector::rows_for_scene,
    view::{App, BarValue, EventCount},
    view_model::UiViewModel,
};

#[derive(Debug, Clone)]
pub struct UiOptions {
    pub midi_path: Option<PathBuf>,
    pub renderer: RendererKind,
    pub start_time: f64,
    pub view_range: f64,
    pub first_key: u8,
    pub last_key: u8,
    pub disable_wgpu: bool,
}

impl Default for UiOptions {
    fn default() -> Self {
        Self {
            midi_path: None,
            renderer: RendererKind::Pfa,
            start_time: 0.0,
            view_range: 8.0,
            first_key: 0,
            last_key: 127,
            disable_wgpu: matches!(
                env::var("MERIDIAN_DISABLE_WGPU").as_deref(),
                Ok("1" | "true" | "yes")
            ),
        }
    }
}

pub fn apply_events_to_app(
    app: &App,
    shared_state: &Arc<Mutex<UiViewModel>>,
    events: &[CoreEvent],
) {
    let snapshot = shared_state
        .lock()
        .expect("shared UI state mutex poisoned")
        .snapshot
        .clone();
    if let Some(state) = snapshot.as_ref() {
        apply_state_to_app(app, shared_state, state);
        for event in events {
            apply_event_overrides_to_app(app, event, state);
        }
    } else {
        for event in events {
            if let CoreEvent::Error { message, .. } = event {
                app.set_status_text(message.clone().into());
            }
        }
    }
}

#[derive(Clone)]
pub struct UiFrameUpdate {
    pub state: StateSnapshot,
    pub visible_notes: usize,
    pub active_keys: usize,
    pub fps_text: String,
}

pub fn apply_frame_update_to_app(
    app: &App,
    shared_state: &Arc<Mutex<UiViewModel>>,
    update: &UiFrameUpdate,
) {
    apply_state_to_app(app, shared_state, &update.state);
    app.set_visible_note_count_text(update.visible_notes.to_string().into());
    app.set_active_keys_text(update.active_keys.to_string().into());
    app.set_fps_text(update.fps_text.clone().into());
    app.set_status_text(status_text(&update.state).into());
}

fn apply_event_overrides_to_app(app: &App, event: &CoreEvent, state: &StateSnapshot) {
    match event {
        CoreEvent::StateSnapshot { .. }
        | CoreEvent::MidiLoaded { .. }
        | CoreEvent::ProcessedMidiAttached { .. }
        | CoreEvent::DisplayCacheAttached { .. }
        | CoreEvent::AudioCacheAttached { .. } => {
            app.set_status_text(status_text(state).into());
            app.set_visible_note_count_text("0".into());
            app.set_active_keys_text("0".into());
        }
        CoreEvent::ProcessedMidiBuilt {
            midi_length,
            total_notes,
            track_count,
            ..
        } => {
            app.set_analysis_note_count_text(total_notes.to_string().into());
            app.set_analysis_track_count_text(track_count.to_string().into());
            app.set_analysis_midi_length_text(format!("{:.3} s", midi_length).into());
        }
        CoreEvent::MidiAnalysis { analysis, .. } => {
            apply_analysis_to_app(app, analysis);
        }
        CoreEvent::FrameProjected { stats, .. } => {
            app.set_visible_note_count_text(stats.visible_notes.to_string().into());
            app.set_active_keys_text(stats.active_keys.to_string().into());
            app.set_status_text(status_text(state).into());
        }
        CoreEvent::FrameSaved { output, .. } => {
            app.set_status_text(format!("Saved frame to {}", output.display()).into());
        }
        CoreEvent::VideoRender { .. }
        | CoreEvent::VideoRenderStatus { .. }
        | CoreEvent::AudioRender { .. }
        | CoreEvent::AudioRenderStatus { .. }
        | CoreEvent::AudioStatus { .. }
        | CoreEvent::MidiLoadProgress { .. }
        | CoreEvent::ParsedMidiLoaded { .. }
        | CoreEvent::DisplayCacheBuilt { .. }
        | CoreEvent::AudioCacheBuilt { .. }
        | CoreEvent::DisplaySessionCreated { .. }
        | CoreEvent::AudioSessionCreated { .. }
        | CoreEvent::MidiAnalysisJob { .. }
        | CoreEvent::MidiAnalysisJobStatus { .. }
        | CoreEvent::MidiFilesProcessed { .. }
        | CoreEvent::MidiProcess { .. }
        | CoreEvent::MidiProcessStatus { .. } => {}
        CoreEvent::DisplaySessionAttached { .. } | CoreEvent::AudioSessionAttached { .. } => {
            app.set_status_text(status_text(state).into());
        }
        CoreEvent::Error { message, .. } => {
            app.set_status_text(message.clone().into());
        }
        CoreEvent::ShutdownComplete => {}
    }
}

fn apply_state_to_app(app: &App, shared_state: &Arc<Mutex<UiViewModel>>, state: &StateSnapshot) {
    let audio_render_status = {
        let mut model = shared_state.lock().expect("shared UI state mutex poisoned");
        model.apply_snapshot(state);
        model.render_jobs.audio.clone()
    };
    app.set_midi_path_text(
        state
            .midi_path
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "No MIDI loaded".into())
            .into(),
    );
    app.set_time_text(format!("{:.3} s", state.current_time).into());
    app.set_length_text(format!("{:.3} s", state.midi_length).into());
    app.set_note_count_text(state.total_notes.to_string().into());
    app.set_view_range_text(format!("{:.1} s", state.view_range).into());
    app.set_current_time_seconds(state.current_time as f32);
    app.set_midi_length_seconds(state.midi_length.max(0.001) as f32);
    app.set_scene_summary_text(scene_summary(&state.scene).into());
    app.set_current_renderer_text(renderer_summary(&state.scene).into());
    app.set_time_space_text(match state.time_space {
        meridian_core::render::DisplayTimeSpace::Time => "time".into(),
        meridian_core::render::DisplayTimeSpace::Tick => "tick".into(),
    });
    app.set_viewport_text(format!("{} x {}", state.viewport_width, state.viewport_height).into());
    app.set_inspector_items(ModelRc::from(std::rc::Rc::new(VecModel::from(
        rows_for_scene(&state.scene),
    ))));
    apply_video_scene_to_app(app, state);
    app.set_play_label(if state.playing {
        "Pause".into()
    } else {
        "Play".into()
    });
    apply_audio_to_app(app, state, &audio_render_status);
}

fn status_text(state: &StateSnapshot) -> String {
    if let Some(path) = &state.midi_path {
        format!(
            "Core session active. UI frontend attached. Rendering {} through the shared event bus.",
            path.display()
        )
    } else {
        "Launch with `meridian-ui --midi <file.mid>` to render a MIDI".into()
    }
}

fn scene_summary(scene: &SceneConfig) -> String {
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
    }
}

fn note_name(config: &NoteProjectorConfig) -> &'static str {
    match config {
        NoteProjectorConfig::Flat(_) => "Flat",
        NoteProjectorConfig::Pfa(_) => "PFA",
    }
}

fn keyboard_name(config: &KeyboardProjectorConfig) -> &'static str {
    match config {
        KeyboardProjectorConfig::Flat(_) => "Flat",
        KeyboardProjectorConfig::Pfa(_) => "PFA",
    }
}

fn height_name(config: &KeyboardHeightSpec) -> &'static str {
    match config {
        KeyboardHeightSpec::ScreenPercent { .. } => "screen %",
        KeyboardHeightSpec::AspectRatio { .. } => "aspect",
    }
}

fn renderer_summary(scene: &SceneConfig) -> &'static str {
    match scene {
        SceneConfig::TwoD(scene) => match (&scene.notes, &scene.keyboard) {
            (NoteProjectorConfig::Pfa(_), KeyboardProjectorConfig::Pfa(_)) => "pfa",
            (NoteProjectorConfig::Flat(_), KeyboardProjectorConfig::Flat(_)) => "flat",
            _ => "mixed",
        },
        SceneConfig::ThreeD(_) => "3d",
    }
}

fn apply_video_scene_to_app(app: &App, state: &StateSnapshot) {
    app.set_video_view_range_value_text(format!("{:.1}", state.view_range).into());
    app.set_video_first_key_text(state.first_key.to_string().into());
    app.set_video_last_key_text(state.last_key.to_string().into());

    match &state.scene {
        SceneConfig::TwoD(config) => {
            match config.keyboard_height {
                KeyboardHeightSpec::ScreenPercent { height } => {
                    app.set_video_keyboard_height_mode_text("screen_percent".into());
                    app.set_video_keyboard_height_value_text(format!("{height:.5}").into());
                }
                KeyboardHeightSpec::AspectRatio { ratio } => {
                    app.set_video_keyboard_height_mode_text("aspect_ratio".into());
                    app.set_video_keyboard_height_value_text(format!("{ratio:.5}").into());
                }
            }

            match &config.notes {
                NoteProjectorConfig::Flat(notes) => {
                    app.set_video_note_same_width_text("off".into());
                    app.set_video_border_width_text("1.0".into());
                    apply_palette_to_app(app, &notes.palette);
                }
                NoteProjectorConfig::Pfa(notes) => {
                    app.set_video_note_same_width_text(on_off(notes.same_width_notes).into());
                    app.set_video_border_width_text(format!("{:.1}", notes.border_width).into());
                    apply_palette_to_app(app, &notes.palette);
                }
            }

            match &config.keyboard {
                KeyboardProjectorConfig::Flat(_) => {
                    app.set_video_keyboard_same_width_text("off".into());
                    app.set_video_middle_c_text("off".into());
                    app.set_video_top_color_text("red".into());
                    app.set_video_top_bar_rgb_text("0.5850, 0.0392, 0.0249".into());
                }
                KeyboardProjectorConfig::Pfa(keyboard) => {
                    app.set_video_keyboard_same_width_text(
                        on_off(keyboard.same_width_notes).into(),
                    );
                    app.set_video_middle_c_text(on_off(keyboard.middle_c).into());
                    app.set_video_top_color_text(top_color_name(keyboard).into());
                    app.set_video_top_bar_rgb_text(
                        format!(
                            "{:.4}, {:.4}, {:.4}",
                            keyboard.top_bar_rgb[0],
                            keyboard.top_bar_rgb[1],
                            keyboard.top_bar_rgb[2]
                        )
                        .into(),
                    );
                }
            }

            apply_ptc_defaults_to_app(app);
        }
        SceneConfig::ThreeD(ThreeDSceneConfig::PianoTrailClassic(config)) => {
            app.set_video_keyboard_height_mode_text("aspect_ratio".into());
            app.set_video_keyboard_height_value_text("0.08494".into());
            app.set_video_note_same_width_text("off".into());
            app.set_video_border_width_text("1.0".into());
            app.set_video_keyboard_same_width_text("off".into());
            app.set_video_middle_c_text("off".into());
            app.set_video_top_color_text("red".into());
            app.set_video_top_bar_rgb_text("0.5850, 0.0392, 0.0249".into());
            apply_palette_to_app(app, &config.palette);

            app.set_video_ptc_same_width_text(on_off(config.same_width_notes).into());
            app.set_video_ptc_fov_text(format!("{:.1}", config.fov.to_degrees()).into());
            app.set_video_ptc_view_height_text(format!("{:.2}", config.view_height).into());
            app.set_video_ptc_view_offset_text(format!("{:.2}", config.view_offset).into());
            app.set_video_ptc_view_pan_text(format!("{:.2}", config.view_pan).into());
            app.set_video_ptc_cam_ang_text(format!("{:.2}", config.cam_ang).into());
            app.set_video_ptc_cam_rot_text(format!("{:.2}", config.cam_rot).into());
            app.set_video_ptc_cam_spin_text(format!("{:.2}", config.cam_spin).into());
            app.set_video_ptc_viewdist_text(format!("{:.1}", config.viewdist).into());
            app.set_video_ptc_viewback_text(format!("{:.2}", config.viewback).into());
            app.set_video_ptc_vertical_notes_text(on_off(config.vertical_notes).into());
            app.set_video_ptc_note_down_speed_text(format!("{:.2}", config.note_down_speed).into());
            app.set_video_ptc_note_up_speed_text(format!("{:.2}", config.note_up_speed).into());
            app.set_video_ptc_box_notes_text(on_off(config.box_notes).into());
            app.set_video_ptc_light_shade_text(on_off(config.light_shade).into());
            app.set_video_ptc_show_keyboard_text(on_off(config.show_keyboard).into());
            app.set_video_ptc_tilt_keys_text(on_off(config.tilt_keys).into());
            app.set_video_ptc_eat_notes_text(on_off(config.eat_notes).into());
            app.set_video_ptc_aura_enabled_text(on_off(config.aura_enabled).into());
            app.set_video_ptc_aura_strength_text(format!("{:.1}", config.aura_strength).into());
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
    }
}

fn apply_ptc_defaults_to_app(app: &App) {
    app.set_video_ptc_same_width_text("on".into());
    app.set_video_ptc_fov_text("60.0".into());
    app.set_video_ptc_view_height_text("0.50".into());
    app.set_video_ptc_view_offset_text("0.40".into());
    app.set_video_ptc_view_pan_text("0.00".into());
    app.set_video_ptc_cam_ang_text("0.56".into());
    app.set_video_ptc_cam_rot_text("0.00".into());
    app.set_video_ptc_cam_spin_text("0.00".into());
    app.set_video_ptc_viewdist_text("14.0".into());
    app.set_video_ptc_viewback_text("0.20".into());
    app.set_video_ptc_vertical_notes_text("off".into());
    app.set_video_ptc_note_down_speed_text("0.60".into());
    app.set_video_ptc_note_up_speed_text("0.20".into());
    app.set_video_ptc_box_notes_text("on".into());
    app.set_video_ptc_light_shade_text("off".into());
    app.set_video_ptc_show_keyboard_text("on".into());
    app.set_video_ptc_tilt_keys_text("on".into());
    app.set_video_ptc_eat_notes_text("off".into());
    app.set_video_ptc_aura_enabled_text("on".into());
    app.set_video_ptc_aura_strength_text("2.0".into());
    app.set_video_ptc_notes_change_size_text("off".into());
    app.set_video_ptc_notes_change_tint_text("on".into());
    app.set_video_ptc_use_vel_text("off".into());
    app.set_video_ptc_aura_image_source_text("builtin".into());
    app.set_video_ptc_aura_image_name_text("ring".into());
    app.set_video_ptc_aura_image_path_text("(aura png)".into());
}

fn apply_palette_to_app(app: &App, palette: &NotePaletteConfig) {
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

fn on_off(value: bool) -> &'static str {
    if value { "on" } else { "off" }
}

fn top_color_name(config: &meridian_core::render::PfaKeyboardProjectorConfig) -> &'static str {
    match config.top_color {
        PfaTopColor::Blue => "blue",
        PfaTopColor::Green => "green",
        PfaTopColor::Red => {
            let red = [0.585, 0.0392, 0.0249];
            if config.top_bar_rgb == red {
                "red"
            } else {
                "custom"
            }
        }
    }
}

fn apply_audio_to_app(app: &App, state: &StateSnapshot, audio_render_status: &AudioRenderStatus) {
    let enabled_soundfonts: Vec<_> = state
        .audio
        .soundfonts
        .iter()
        .filter(|soundfont| soundfont.enabled)
        .collect();
    let primary_soundfont = enabled_soundfonts
        .first()
        .map(|soundfont| file_name_or_full(&soundfont.path))
        .unwrap_or_else(|| "(none)".into());
    let soundfont_count = enabled_soundfonts.len();
    let sample_rate = state
        .audio_status
        .stream_params
        .map(|params| params.sample_rate)
        .unwrap_or(state.audio.xsynth.render.audio_params.sample_rate);
    let channels = state
        .audio_status
        .stream_params
        .map(|params| params.channels.count())
        .unwrap_or(state.audio.xsynth.render.audio_params.channels.count());
    let engine_detail = format!(
        "{} / FX {}",
        state
            .audio
            .soundfonts
            .first()
            .map(|soundfont| {
                if soundfont.options.use_effects {
                    "Effects on"
                } else {
                    "Effects off"
                }
            })
            .unwrap_or("Default"),
        if state.audio.xsynth.render.use_limiter {
            "limiter on"
        } else {
            "limiter off"
        }
    );
    let interpolation_text = state
        .audio
        .soundfonts
        .first()
        .map(|soundfont| format!("{:?}", soundfont.options.interpolator).to_lowercase())
        .unwrap_or_else(|| "nearest".into());
    let soundfont_enabled = state
        .audio
        .soundfonts
        .first()
        .is_some_and(|soundfont| soundfont.enabled);
    let effects_text = state
        .audio
        .soundfonts
        .first()
        .map(|soundfont| {
            if soundfont.options.use_effects {
                "on"
            } else {
                "off"
            }
        })
        .unwrap_or("off");
    let attack_curve = state
        .audio
        .soundfonts
        .first()
        .map(|soundfont| curve_name(soundfont.options.vol_envelope_options.attack_curve))
        .unwrap_or("exponential");
    let decay_curve = state
        .audio
        .soundfonts
        .first()
        .map(|soundfont| curve_name(soundfont.options.vol_envelope_options.decay_curve))
        .unwrap_or("linear");
    let release_curve = state
        .audio
        .soundfonts
        .first()
        .map(|soundfont| curve_name(soundfont.options.vol_envelope_options.release_curve))
        .unwrap_or("linear");
    let ignore_range_text = match *state.audio.xsynth.config.ignore_range.end() {
        0 => "off".to_string(),
        end => end.to_string(),
    };

    app.set_audio_backend_text(match state.audio.backend {
        AudioBackend::None => "none".into(),
        AudioBackend::Xsynth => "xsynth".into(),
    });
    app.set_audio_engine_status_text(if state.audio_status.active {
        "Running".into()
    } else {
        "Idle".into()
    });
    app.set_audio_supports_44100(state.audio_status.supports_44100_hz);
    app.set_audio_supports_48000(state.audio_status.supports_48000_hz);
    app.set_audio_supports_88200(state.audio_status.supports_88200_hz);
    app.set_audio_supports_96000(state.audio_status.supports_96000_hz);
    app.set_audio_supports_176400(state.audio_status.supports_176400_hz);
    app.set_audio_supports_192000(state.audio_status.supports_192000_hz);
    app.set_audio_supports_mono(state.audio_status.supports_mono);
    app.set_audio_supports_stereo(state.audio_status.supports_stereo);
    app.set_audio_output_stream_text(format!("{channels} ch @ {sample_rate} Hz").into());
    app.set_audio_sample_rate_text(sample_rate.to_string().into());
    app.set_audio_channel_count_text(if channels == 1 {
        "mono".into()
    } else {
        "stereo".into()
    });
    app.set_audio_render_window_text(
        format!("{:.1} ms", state.audio.xsynth.config.render_window_ms).into(),
    );
    app.set_audio_soundfont_text(primary_soundfont.into());
    app.set_audio_soundfont_count_text(format!("{soundfont_count} loaded").into());
    app.set_audio_soundfont_enabled(soundfont_enabled);
    app.set_audio_engine_detail_text(engine_detail.into());
    app.set_audio_effects_text(effects_text.into());
    app.set_audio_interpolation_text(interpolation_text.into());
    app.set_audio_voice_count_text(
        state
            .audio_status
            .voice_count
            .map(|count| count.to_string())
            .unwrap_or_else(|| "—".into())
            .into(),
    );
    app.set_audio_layer_limit_text(if state.audio.xsynth.limit_layers {
        state.audio.xsynth.layers.to_string().into()
    } else {
        "off".into()
    });
    app.set_audio_limiter_text(if state.audio.xsynth.render.use_limiter {
        "on".into()
    } else {
        "off".into()
    });
    app.set_audio_threading_text(
        thread_count_name(state.audio.xsynth.config.multithreading).into(),
    );
    app.set_audio_ignore_range_text(ignore_range_text.into());
    app.set_audio_attack_curve_text(attack_curve.into());
    app.set_audio_decay_curve_text(decay_curve.into());
    app.set_audio_release_curve_text(release_curve.into());
    app.set_audio_parallelism_text(
        format!(
            "{:?} / {:?}",
            state.audio.xsynth.render.parallelism.channel,
            state.audio.xsynth.render.parallelism.key
        )
        .into(),
    );
    apply_audio_render_status_to_app(app, audio_render_status);
}

fn curve_name(curve: EnvelopeCurveType) -> &'static str {
    match curve {
        EnvelopeCurveType::Linear => "linear",
        EnvelopeCurveType::Exponential => "exponential",
    }
}

fn thread_count_name(threading: ThreadCount) -> &'static str {
    match threading {
        ThreadCount::None => "none",
        ThreadCount::Auto => "auto",
        ThreadCount::Manual(4) => "4",
        ThreadCount::Manual(_) => "manual",
    }
}

fn apply_audio_render_status_to_app(app: &App, status: &AudioRenderStatus) {
    match status {
        AudioRenderStatus::Idle => {
            app.set_audio_render_progress(0.0);
            app.set_audio_render_status("Idle".into());
            app.set_audio_render_elapsed("—".into());
            app.set_audio_render_output_text("output.wav".into());
        }
        AudioRenderStatus::Running {
            output,
            total_events,
            event_index,
            rendered_seconds,
            ..
        } => {
            let progress = if *total_events == 0 {
                0.0
            } else {
                *event_index as f32 / *total_events as f32
            };
            app.set_audio_render_progress(progress);
            app.set_audio_render_status("Rendering WAV".into());
            app.set_audio_render_elapsed(format!("{rendered_seconds:.1} s").into());
            app.set_audio_render_output_text(file_name_or_full(output).into());
        }
        AudioRenderStatus::Cancelling {
            output,
            total_events,
            event_index,
            rendered_seconds,
            ..
        } => {
            let progress = if *total_events == 0 {
                0.0
            } else {
                *event_index as f32 / *total_events as f32
            };
            app.set_audio_render_progress(progress);
            app.set_audio_render_status("Cancelling render".into());
            app.set_audio_render_elapsed(format!("{rendered_seconds:.1} s").into());
            app.set_audio_render_output_text(file_name_or_full(output).into());
        }
    }
}

fn file_name_or_full(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| path.display().to_string())
}

fn apply_analysis_to_app(app: &App, analysis: &MidiAnalysisData) {
    // ── Overview ──
    app.set_analysis_note_count_text(format_number(analysis.total_notes).into());
    app.set_analysis_midi_length_text(format_duration(analysis.midi_length).into());

    let first_key = analysis
        .key_note_counts
        .iter()
        .position(|count| *count > 0)
        .unwrap_or(0);
    let last_key = analysis
        .key_note_counts
        .iter()
        .rposition(|count| *count > 0)
        .unwrap_or(0);
    app.set_analysis_key_range_text(
        format!(
            "{} – {} ({})",
            midi_note_name(first_key),
            midi_note_name(last_key),
            last_key - first_key + 1
        )
        .into(),
    );

    // ── Note density (computed from buckets) ──
    let bucket_width = if analysis.buckets.len() > 1 {
        analysis.buckets[1].time_seconds - analysis.buckets[0].time_seconds
    } else {
        analysis.midi_length.max(0.5)
    }
    .max(0.001);
    let peak_nps = analysis
        .buckets
        .iter()
        .map(|bucket| bucket.note_starts as f64 / bucket_width)
        .fold(0.0, f64::max);
    let avg_nps = if analysis.midi_length > 0.0 {
        analysis.total_notes as f64 / analysis.midi_length
    } else {
        analysis.total_notes as f64
    };
    app.set_analysis_note_density_text(
        format!("avg {:.1}/s peak {:.1}/s", avg_nps, peak_nps).into(),
    );
    app.set_analysis_peak_nps_text(format!("{:.1}", peak_nps).into());
    app.set_analysis_avg_nps_text(format!("{:.1}", avg_nps).into());

    // ── File metrics ──
    app.set_analysis_file_size_text(format_bytes(analysis.file.source_bytes).into());
    app.set_analysis_gzip_size_text(format_bytes(analysis.file.gzip_bytes).into());
    app.set_analysis_gzip_ratio_text(format!("{:.1}%", analysis.file.gzip_ratio * 100.0).into());
    app.set_analysis_format_text(format!("Type {}", analysis.file.format).into());
    app.set_analysis_declared_tracks_text(analysis.file.declared_track_count.to_string().into());
    app.set_analysis_actual_tracks_text(analysis.file.actual_track_count.to_string().into());
    app.set_analysis_track_count_text(analysis.file.actual_track_count.to_string().into());
    app.set_analysis_ticks_per_quarter_text(
        analysis
            .file
            .ticks_per_quarter
            .map(|t| t.to_string())
            .unwrap_or_else(|| "—".into())
            .into(),
    );
    app.set_analysis_total_events_text(format_number(analysis.file.total_event_count).into());

    // ── Tempo metrics ──
    app.set_analysis_initial_bpm_text(format!("{:.1}", analysis.tempo.initial_bpm).into());
    app.set_analysis_min_bpm_text(format!("{:.1}", analysis.tempo.min_bpm).into());
    app.set_analysis_max_bpm_text(format!("{:.1}", analysis.tempo.max_bpm).into());
    app.set_analysis_avg_bpm_text(format!("{:.1}", analysis.tempo.avg_bpm_weighted_by_time).into());
    app.set_analysis_tempo_text(
        format!(
            "{:.0} BPM ({:.0}–{:.0})",
            analysis.tempo.avg_bpm_weighted_by_time, analysis.tempo.min_bpm, analysis.tempo.max_bpm
        )
        .into(),
    );
    app.set_analysis_time_signature_text("—".into()); // Not in analysis data

    // ── Note metrics ──
    app.set_analysis_avg_note_length_text(
        format_duration_short(analysis.notes.avg_note_length_seconds).into(),
    );
    app.set_analysis_min_note_length_text(
        format_duration_short(analysis.notes.min_note_length_seconds).into(),
    );
    app.set_analysis_max_note_length_text(
        format_duration_short(analysis.notes.max_note_length_seconds).into(),
    );
    app.set_analysis_total_note_duration_text(
        format_duration(analysis.notes.total_note_duration_seconds).into(),
    );
    app.set_analysis_max_polyphony_text(analysis.notes.max_simultaneous_notes.to_string().into());
    app.set_analysis_avg_polyphony_text(
        format!("{:.1}", analysis.notes.avg_simultaneous_notes).into(),
    );
    app.set_analysis_unique_onsets_text(format_number(analysis.notes.unique_onset_count).into());
    app.set_analysis_avg_notes_per_onset_text(
        format!("{:.2}", analysis.notes.avg_notes_per_onset).into(),
    );

    // ── Average velocity (computed from histogram) ──
    let total_velocity_notes: u64 = analysis.notes.velocity_note_on_counts.iter().sum();
    let weighted_velocity: u64 = analysis
        .notes
        .velocity_note_on_counts
        .iter()
        .enumerate()
        .map(|(i, &c)| i as u64 * c)
        .sum();
    let avg_velocity = if total_velocity_notes > 0 {
        weighted_velocity as f64 / total_velocity_notes as f64
    } else {
        0.0
    };
    app.set_analysis_avg_velocity_text(format!("{:.0}", avg_velocity).into());

    // ── Summary (block analysis) ──
    app.set_analysis_total_blocks_text(format_number(analysis.summary.total_blocks).into());
    app.set_analysis_keys_with_notes_text(analysis.summary.keys_with_notes.to_string().into());
    app.set_analysis_densest_key_text(midi_note_name(analysis.summary.densest_key).into());
    app.set_analysis_densest_key_notes_text(
        format_number(analysis.summary.densest_key_notes).into(),
    );
    app.set_analysis_max_blocks_per_key_text(
        format_number(analysis.summary.max_blocks_per_key as u64).into(),
    );
    app.set_analysis_max_notes_in_block_text(
        format_number(analysis.summary.max_notes_in_block as u64).into(),
    );

    // ── Histograms ──
    app.set_analysis_key_histogram(build_bar_model(&analysis.key_note_counts, |i| {
        midi_note_name(i)
    }));
    app.set_analysis_velocity_histogram(build_bar_model(
        &analysis.notes.velocity_note_on_counts,
        |i| i.to_string(),
    ));
    app.set_analysis_pitch_class_histogram(build_bar_model(
        &analysis.notes.pitch_class_note_counts,
        |i| {
            const NAMES: [&str; 12] = [
                "C", "C♯", "D", "D♯", "E", "F", "F♯", "G", "G♯", "A", "A♯", "B",
            ];
            NAMES.get(i).unwrap_or(&"?").to_string()
        },
    ));
    app.set_analysis_channel_histogram(build_bar_model(&analysis.notes.channel_note_counts, |i| {
        i.to_string()
    }));
    app.set_analysis_track_histogram(build_bar_model(&analysis.notes.track_note_counts, |i| {
        format!("T{}", i)
    }));

    // ── Density timeline ──
    let peak_bucket = analysis
        .buckets
        .iter()
        .map(|b| b.note_starts)
        .max()
        .unwrap_or(1)
        .max(1) as f32;
    let density_bars: Vec<BarValue> = analysis
        .buckets
        .iter()
        .map(|b| BarValue {
            value: b.note_starts as f32 / peak_bucket,
            label: SharedString::from(format!("{:.0}s", b.time_seconds)),
            count: b.note_starts as i32,
        })
        .collect();
    app.set_analysis_density_timeline(ModelRc::from(std::rc::Rc::new(VecModel::from(
        density_bars,
    ))));

    // ── Event breakdown ──
    let events = &analysis.events;
    let event_pairs: Vec<(&str, u64)> = vec![
        ("Note On", events.note_on_events),
        ("Note Off", events.note_off_events),
        ("Zero-Vel Note On", events.zero_velocity_note_on_events),
        ("Program Change", events.program_change_events),
        ("Control Change", events.control_change_events),
        ("Pitch Bend", events.pitch_bend_events),
        ("Channel Pressure", events.channel_pressure_events),
        ("Polyphonic Pressure", events.polyphonic_pressure_events),
        ("SysEx", events.sysex_events),
        ("Text", events.text_events),
        ("Lyric", events.lyric_events),
        ("Marker", events.marker_events),
        ("Cue Point", events.cue_point_events),
        ("Track Name", events.track_name_events),
        ("Instrument Name", events.instrument_name_events),
        ("Tempo", events.tempo_events),
        ("Time Signature", events.time_signature_events),
        ("Key Signature", events.key_signature_events),
    ];
    let max_event_count = event_pairs
        .iter()
        .map(|(_, c)| *c)
        .max()
        .unwrap_or(1)
        .max(1);
    let event_counts: Vec<EventCount> = event_pairs
        .into_iter()
        .filter(|(_, count)| *count > 0)
        .map(|(name, count)| EventCount {
            name: SharedString::from(name),
            count: count as i32,
            fraction: count as f32 / max_event_count as f32,
        })
        .collect();
    app.set_analysis_event_breakdown(ModelRc::from(std::rc::Rc::new(VecModel::from(
        event_counts,
    ))));
    app.set_analysis_event_total_text(format_number(analysis.file.total_event_count).into());
}

// ── Formatting helpers ──

fn format_number(n: impl Into<u64>) -> String {
    let n = n.into();
    if n >= 1_000_000 {
        format!("{:.2}M", n as f64 / 1_000_000.0)
    } else if n >= 10_000 {
        format!("{:.1}K", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}

fn format_bytes(bytes: u64) -> String {
    if bytes >= 1_048_576 {
        format!("{:.1} MB", bytes as f64 / 1_048_576.0)
    } else if bytes >= 1_024 {
        format!("{:.1} KB", bytes as f64 / 1_024.0)
    } else {
        format!("{} B", bytes)
    }
}

fn format_duration(seconds: f64) -> String {
    if seconds >= 3600.0 {
        let h = (seconds / 3600.0).floor() as u64;
        let m = ((seconds % 3600.0) / 60.0).floor() as u64;
        let s = seconds % 60.0;
        format!("{}h {:02}m {:04.1}s", h, m, s)
    } else if seconds >= 60.0 {
        let m = (seconds / 60.0).floor() as u64;
        let s = seconds % 60.0;
        format!("{}m {:04.1}s", m, s)
    } else {
        format!("{:.3}s", seconds)
    }
}

fn format_duration_short(seconds: f64) -> String {
    if seconds >= 1.0 {
        format!("{:.2}s", seconds)
    } else if seconds >= 0.001 {
        format!("{:.1}ms", seconds * 1_000.0)
    } else {
        format!("{:.0}µs", seconds * 1_000_000.0)
    }
}

fn midi_note_name(key: usize) -> String {
    const NAMES: [&str; 12] = [
        "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
    ];
    let octave = key as i32 / 12 - 1;
    let name = NAMES[key % 12];
    format!("{}{}", name, octave)
}

fn build_bar_model(counts: &[u64], label_fn: impl Fn(usize) -> String) -> ModelRc<BarValue> {
    let max = counts.iter().copied().max().unwrap_or(1).max(1) as f32;
    let bars: Vec<BarValue> = counts
        .iter()
        .enumerate()
        .map(|(i, &c)| BarValue {
            value: c as f32 / max,
            label: SharedString::from(label_fn(i)),
            count: c as i32,
        })
        .collect();
    ModelRc::from(std::rc::Rc::new(VecModel::from(bars)))
}
