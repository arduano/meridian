use meridian_core::audio::{AudioBackend, EnvelopeCurveType, ThreadCount};
use meridian_core::protocol::{AudioRenderStatus, StateSnapshot};

use super::super::view::App;
use super::formatting::file_name_or_full;

pub(super) fn apply_audio_to_app(
    app: &App,
    state: &StateSnapshot,
    audio_render_status: &AudioRenderStatus,
) {
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
    let ignore_range_text = keep_velocity_text(&state.audio.xsynth.config.ignore_range);

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

pub(super) fn curve_name(curve: EnvelopeCurveType) -> &'static str {
    match curve {
        EnvelopeCurveType::Linear => "linear",
        EnvelopeCurveType::Exponential => "exponential",
    }
}

pub(super) fn thread_count_name(threading: ThreadCount) -> &'static str {
    match threading {
        ThreadCount::None => "none",
        ThreadCount::Auto => "auto",
        ThreadCount::Manual(4) => "4",
        ThreadCount::Manual(_) => "manual",
    }
}

pub(super) fn keep_velocity_text(ignore_range: &std::ops::RangeInclusive<u8>) -> String {
    match *ignore_range.end() {
        0 => "off".to_string(),
        end => end.saturating_add(1).min(127).to_string(),
    }
}

pub(super) fn apply_audio_render_status_to_app(app: &App, status: &AudioRenderStatus) {
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
