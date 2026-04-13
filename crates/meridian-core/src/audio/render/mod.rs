mod config;
mod encode;
mod events;
mod render_loop;
mod renderer;
mod writer;

pub use config::AudioRenderConfig;
pub use events::AudioRenderEvent;

use std::{path::Path, sync::atomic::AtomicBool};

use crate::{
    MeridianError,
    midi::{MidiCacheStack, audio_cache::InRamAudioCache},
    protocol::{AudioOutputFormat, AudioRenderJobId, VideoAudioConfig, VideoAudioProgress},
};

use super::{AudioBackend, AudioConfig, soundfont_cache::SoundfontCache};
use crate::audio::MeridianSoundfont;

use config::resolve_render_settings;
use encode::render_encoded_audio;
use render_loop::{AudioRenderLoopResult, run_audio_render_loop};
use renderer::OfflineAudioRenderer;

pub(crate) use config::resolve_video_audio_settings;

pub(crate) fn render_audio_pipe_from_cache(
    events: &InRamAudioCache,
    audio_config: &AudioConfig,
    soundfont_cache: &SoundfontCache,
    config: &VideoAudioConfig,
    pipe_path: &Path,
    cancel: &AtomicBool,
    mut on_progress: impl FnMut(VideoAudioProgress),
) -> Result<(), MeridianError> {
    let (sample_rate, channels, use_limiter) = resolve_video_audio_settings(audio_config, config)?;
    let audio_params = xsynth_core::AudioStreamParams::new(
        sample_rate,
        xsynth_core::ChannelCount::from_count(channels).expect("validated channel count"),
    );
    let mut render_audio_config = audio_config.clone();
    if !config.soundfonts.is_empty() {
        render_audio_config.soundfonts = config
            .soundfonts
            .iter()
            .cloned()
            .map(|path| MeridianSoundfont {
                path: Some(path),
                ..MeridianSoundfont::default()
            })
            .collect();
    }
    let renderer = OfflineAudioRenderer::new_raw_pipe(
        &render_audio_config,
        soundfont_cache,
        pipe_path,
        audio_params,
        use_limiter,
        cancel,
    )?;
    let noop_config = AudioRenderConfig {
        output: pipe_path.to_path_buf(),
        ..AudioRenderConfig::default()
    };
    let total_events = events.events().len();
    on_progress(VideoAudioProgress {
        total_events,
        event_index: 0,
        rendered_seconds: 0.0,
    });
    let result = run_audio_render_loop(
        events,
        &render_audio_config,
        renderer,
        &noop_config,
        AudioRenderJobId(0),
        cancel,
        &mut |event| {
            if let AudioRenderEvent::RenderProgress {
                total_events,
                event_index,
                rendered_seconds,
                ..
            } = event
            {
                on_progress(VideoAudioProgress {
                    total_events,
                    event_index,
                    rendered_seconds,
                });
            }
        },
    )?;
    match result {
        AudioRenderLoopResult::Finished {
            rendered_seconds, ..
        } => {
            on_progress(VideoAudioProgress {
                total_events,
                event_index: total_events,
                rendered_seconds,
            });
            Ok(())
        }
        AudioRenderLoopResult::Cancelled => Ok(()),
    }
}

pub fn render_audio(
    midi_cache: &MidiCacheStack,
    audio_config: &AudioConfig,
    soundfont_cache: &SoundfontCache,
    config: &AudioRenderConfig,
    job_id: AudioRenderJobId,
    cancel: &AtomicBool,
    on_event: impl FnMut(AudioRenderEvent),
) -> Result<(), MeridianError> {
    let events = midi_cache.audio_cache()?;
    render_audio_from_cache(
        events.as_ref(),
        audio_config,
        soundfont_cache,
        config,
        job_id,
        cancel,
        on_event,
    )
}

pub fn render_audio_from_cache(
    events: &InRamAudioCache,
    audio_config: &AudioConfig,
    soundfont_cache: &SoundfontCache,
    config: &AudioRenderConfig,
    job_id: AudioRenderJobId,
    cancel: &AtomicBool,
    mut on_event: impl FnMut(AudioRenderEvent),
) -> Result<(), MeridianError> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        render_audio_inner(
            events,
            audio_config,
            soundfont_cache,
            config,
            job_id,
            cancel,
            &mut on_event,
        )
    }))
    .map_err(|_| MeridianError::Platform("xsynth panicked during offline audio render".into()))?
    .inspect_err(|error| {
        on_event(AudioRenderEvent::RenderFailed {
            message: error.to_string(),
        });
    })
}

pub fn render_audio_to_wav(
    midi_cache: &MidiCacheStack,
    audio_config: &AudioConfig,
    soundfont_cache: &SoundfontCache,
    render_config: &AudioRenderConfig,
    callback: impl FnMut(AudioRenderEvent),
) -> Result<(), MeridianError> {
    let cancel = AtomicBool::new(false);
    render_audio(
        midi_cache,
        audio_config,
        soundfont_cache,
        render_config,
        AudioRenderJobId(0),
        &cancel,
        callback,
    )
}

fn render_audio_inner(
    events: &InRamAudioCache,
    audio_config: &AudioConfig,
    soundfont_cache: &SoundfontCache,
    render_config: &AudioRenderConfig,
    job_id: AudioRenderJobId,
    cancel: &AtomicBool,
    mut callback: impl FnMut(AudioRenderEvent),
) -> Result<(), MeridianError> {
    render_config.validate()?;

    if matches!(audio_config.backend, AudioBackend::None) {
        return Err(MeridianError::Platform(
            "audio backend 'none' cannot render audio".into(),
        ));
    }

    let settings = resolve_render_settings(audio_config, render_config)?;

    callback(AudioRenderEvent::RenderStarted {
        job_id,
        output: render_config.output.clone(),
        sample_rate: settings.sample_rate,
        channels: settings.channels,
        total_events: events.events().len(),
    });

    let outcome = match render_config.format {
        AudioOutputFormat::Wav => {
            let renderer = OfflineAudioRenderer::new_wav(
                audio_config,
                soundfont_cache,
                &render_config.output,
                settings.audio_params()?,
                settings.use_limiter,
            )?;
            run_audio_render_loop(
                events,
                audio_config,
                renderer,
                render_config,
                job_id,
                cancel,
                &mut callback,
            )?
        }
        AudioOutputFormat::Flac | AudioOutputFormat::Mp3 => render_encoded_audio(
            events,
            audio_config,
            soundfont_cache,
            render_config,
            settings,
            job_id,
            cancel,
            &mut callback,
        )?,
    };

    if let AudioRenderLoopResult::Finished {
        frames_written,
        rendered_seconds,
    } = outcome
    {
        callback(AudioRenderEvent::RenderFinished {
            job_id,
            output: render_config.output.clone(),
            frames_written,
            rendered_seconds,
        });
    }

    Ok(())
}
