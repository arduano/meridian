use std::{
    ops::RangeInclusive,
    sync::atomic::{AtomicBool, Ordering},
};

use xsynth_core::{
    channel::{ChannelAudioEvent, ChannelEvent, ControlEvent},
    channel_group::SynthEvent,
};

use crate::{MeridianError, midi::audio_cache::InRamAudioCache, protocol::AudioRenderJobId};

use super::{
    AudioConfig, config::AudioRenderConfig, events::AudioRenderEvent,
    renderer::OfflineAudioRenderer,
};

pub(crate) enum AudioRenderLoopResult {
    Finished {
        frames_written: u64,
        rendered_seconds: f64,
    },
    Cancelled,
}

pub(crate) fn run_audio_render_loop(
    events: &InRamAudioCache,
    audio_config: &AudioConfig,
    mut renderer: OfflineAudioRenderer,
    render_config: &AudioRenderConfig,
    job_id: AudioRenderJobId,
    cancel: &AtomicBool,
    callback: &mut impl FnMut(AudioRenderEvent),
) -> Result<AudioRenderLoopResult, MeridianError> {
    let mut current_time = 0.0;
    let total_events = events.events().len();
    let total_duration_seconds = events
        .events()
        .last()
        .map(|event| event.time)
        .unwrap_or(0.0);
    let progress_stride_seconds = progress_stride_seconds(total_duration_seconds);
    let mut next_progress_seconds = progress_stride_seconds;
    for (event_index, event) in events.events().iter().enumerate() {
        if cancel.load(Ordering::SeqCst) {
            callback(AudioRenderEvent::RenderCancelled {
                job_id,
                output: render_config.output.clone(),
                event_index,
                total_events,
                rendered_seconds: current_time,
                frames_written: renderer.frames_written(),
            });
            return Ok(AudioRenderLoopResult::Cancelled);
        }

        while current_time + f64::EPSILON < event.time {
            if cancel.load(Ordering::SeqCst) {
                callback(AudioRenderEvent::RenderCancelled {
                    job_id,
                    output: render_config.output.clone(),
                    event_index,
                    total_events,
                    rendered_seconds: current_time,
                    frames_written: renderer.frames_written(),
                });
                return Ok(AudioRenderLoopResult::Cancelled);
            }

            let next_time = next_progress_seconds.min(event.time);
            let delta = (next_time - current_time).max(0.0);
            if delta <= 0.0 {
                break;
            }

            renderer.render_batch(delta)?;
            current_time = next_time;

            if current_time + f64::EPSILON >= next_progress_seconds {
                emit_audio_render_progress(
                    callback,
                    job_id,
                    event_index,
                    total_events,
                    current_time,
                    current_time,
                    &renderer,
                );
                next_progress_seconds += progress_stride_seconds;
            }
        }

        for packed in event.iter_events() {
            dispatch_packed_event(
                &mut renderer,
                packed,
                &audio_config.xsynth.config.ignore_range,
            );
        }

        if event_index == 0 || event_index + 1 == total_events {
            emit_audio_render_progress(
                callback,
                job_id,
                event_index + 1,
                total_events,
                event.time,
                current_time,
                &renderer,
            );
        }
    }

    renderer.send_event(SynthEvent::AllChannels(ChannelEvent::Audio(
        ChannelAudioEvent::AllNotesOff,
    )));
    renderer.send_event(SynthEvent::AllChannels(ChannelEvent::Audio(
        ChannelAudioEvent::ResetControl,
    )));
    let frames_written = renderer.finalize()?;
    Ok(AudioRenderLoopResult::Finished {
        frames_written,
        rendered_seconds: current_time,
    })
}

fn emit_audio_render_progress(
    callback: &mut impl FnMut(AudioRenderEvent),
    job_id: AudioRenderJobId,
    event_index: usize,
    total_events: usize,
    time_seconds: f64,
    rendered_seconds: f64,
    renderer: &OfflineAudioRenderer,
) {
    callback(AudioRenderEvent::RenderProgress {
        job_id,
        event_index,
        total_events,
        time_seconds,
        rendered_seconds,
        frames_written: renderer.frames_written(),
        voice_count: renderer.voice_count(),
    });
}

fn progress_stride_seconds(total_duration_seconds: f64) -> f64 {
    (total_duration_seconds / 400.0).clamp(0.05, 1.0)
}

fn dispatch_packed_event(
    renderer: &mut OfflineAudioRenderer,
    packed: u32,
    ignore_range: &RangeInclusive<u8>,
) {
    let status = (packed & 0xFF) as u8;
    let data1 = ((packed >> 8) & 0xFF) as u8;
    let data2 = ((packed >> 16) & 0xFF) as u8;
    let event = status & 0xF0;
    let channel = (status & 0x0F) as u32;
    match event {
        0x80 => renderer.send_event(SynthEvent::Channel(
            channel,
            ChannelEvent::Audio(ChannelAudioEvent::NoteOff { key: data1 }),
        )),
        0x90 if !ignore_range.contains(&data2) => {
            renderer.send_event(SynthEvent::Channel(
                channel,
                ChannelEvent::Audio(ChannelAudioEvent::NoteOn {
                    key: data1,
                    vel: data2,
                }),
            ));
        }
        0xB0 => renderer.send_event(SynthEvent::Channel(
            channel,
            ChannelEvent::Audio(ChannelAudioEvent::Control(ControlEvent::Raw(data1, data2))),
        )),
        0xC0 => renderer.send_event(SynthEvent::Channel(
            channel,
            ChannelEvent::Audio(ChannelAudioEvent::ProgramChange(data1)),
        )),
        0xE0 => {
            let raw = (data1 as i16) | ((data2 as i16) << 7);
            let pitch = raw - 8192;
            renderer.send_event(SynthEvent::Channel(
                channel,
                ChannelEvent::Audio(ChannelAudioEvent::Control(ControlEvent::PitchBendValue(
                    pitch as f32 / 8192.0,
                ))),
            ));
        }
        _ => {}
    }
}
