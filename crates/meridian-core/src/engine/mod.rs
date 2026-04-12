mod analysis_job;
mod audio_render_job;
mod core_state;
mod job_runtime;
mod midi_process_job;
mod resource_types;
mod resources;
mod state_ops;
mod support;
mod video_render_job;

use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    thread,
};

use flume::{Receiver, Sender};

use crate::{
    audio::AudioRenderEvent,
    error::MeridianError,
    protocol::{
        CoreCommand, CoreEvent, MidiAnalysisJobEvent, MidiProcessEvent, RenderedFrame,
        VideoRenderEvent,
    },
};

pub use support::{error_code, event_to_error};

#[derive(Debug)]
enum RequestMessage {
    Command {
        command: CoreCommand,
        reply: Sender<CoreResponse>,
    },
    RenderFrame {
        viewport_width: Option<u32>,
        viewport_height: Option<u32>,
        reply: Sender<Result<RenderedFrame, MeridianError>>,
    },
    VideoRenderUpdate {
        event: VideoRenderEvent,
    },
    AudioRenderUpdate {
        event: AudioRenderEvent,
    },
    MidiProcessUpdate {
        event: MidiProcessEvent,
    },
    AnalysisJobUpdate {
        event: MidiAnalysisJobEvent,
    },
}

pub type CoreResponse = Vec<CoreEvent>;

#[derive(Clone)]
pub struct CoreHandle {
    sender: Sender<RequestMessage>,
    subscribers: Arc<Mutex<Vec<Sender<CoreEvent>>>>,
    midi_load_generation: Arc<AtomicU64>,
}

pub fn spawn_core() -> CoreHandle {
    let (sender, receiver) = flume::unbounded();
    let subscribers = Arc::new(Mutex::new(Vec::new()));
    let midi_load_generation = Arc::new(AtomicU64::new(0));
    let core_handle = CoreHandle {
        sender: sender.clone(),
        subscribers: Arc::clone(&subscribers),
        midi_load_generation: Arc::clone(&midi_load_generation),
    };
    thread::spawn(move || {
        let mut core = core_state::CoreState::new(sender, subscribers, midi_load_generation);
        core.run(receiver);
    });
    core_handle
}

impl CoreHandle {
    pub fn request(&self, command: CoreCommand) -> Result<CoreResponse, MeridianError> {
        let (reply_tx, reply_rx) = flume::bounded(1);
        self.sender
            .send(RequestMessage::Command {
                command,
                reply: reply_tx,
            })
            .map_err(|_| MeridianError::Transport("core request channel closed".into()))?;
        reply_rx
            .recv()
            .map_err(|_| MeridianError::Transport("core reply channel closed".into()))
    }

    pub fn render_frame(
        &self,
        viewport_width: Option<u32>,
        viewport_height: Option<u32>,
    ) -> Result<RenderedFrame, MeridianError> {
        let (reply_tx, reply_rx) = flume::bounded(1);
        self.sender
            .send(RequestMessage::RenderFrame {
                viewport_width,
                viewport_height,
                reply: reply_tx,
            })
            .map_err(|_| MeridianError::Transport("core request channel closed".into()))?;
        reply_rx
            .recv()
            .map_err(|_| MeridianError::Transport("core reply channel closed".into()))?
    }

    pub fn subscribe_events(&self) -> Receiver<CoreEvent> {
        let (sender, receiver) = flume::unbounded();
        if let Ok(mut subscribers) = self.subscribers.lock() {
            subscribers.push(sender);
        }
        receiver
    }

    pub fn cancel_midi_loads(&self) {
        self.midi_load_generation.fetch_add(1, Ordering::SeqCst);
    }

    pub(crate) fn current_midi_load_generation(&self) -> u64 {
        self.midi_load_generation.load(Ordering::SeqCst)
    }

    pub(crate) fn publish_video_event(&self, event: VideoRenderEvent) -> Result<(), MeridianError> {
        self.sender
            .send(RequestMessage::VideoRenderUpdate { event })
            .map_err(|_| MeridianError::Transport("core request channel closed".into()))
    }

    pub(crate) fn publish_audio_event(&self, event: AudioRenderEvent) -> Result<(), MeridianError> {
        self.sender
            .send(RequestMessage::AudioRenderUpdate { event })
            .map_err(|_| MeridianError::Transport("core request channel closed".into()))
    }

    pub(crate) fn publish_midi_process_event(
        &self,
        event: MidiProcessEvent,
    ) -> Result<(), MeridianError> {
        self.sender
            .send(RequestMessage::MidiProcessUpdate { event })
            .map_err(|_| MeridianError::Transport("core request channel closed".into()))
    }

    pub(crate) fn publish_analysis_job_event(
        &self,
        event: MidiAnalysisJobEvent,
    ) -> Result<(), MeridianError> {
        self.sender
            .send(RequestMessage::AnalysisJobUpdate { event })
            .map_err(|_| MeridianError::Transport("core request channel closed".into()))
    }
}

#[cfg(test)]
mod tests {
    use std::{path::PathBuf, thread, time::Duration};

    use crate::{
        audio::{AudioBackend, AudioConfig},
        midi::test_support::{note_off, note_on, write_toolkit_midi},
        protocol::{CoreCommand, CoreErrorCode, CoreEvent, StateSnapshot},
        render::DisplayTimeSpace,
    };

    use super::{CoreHandle, spawn_core};

    fn midi_fixture(relative_path: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../assets/midis")
            .join(relative_path)
    }

    fn test_core() -> CoreHandle {
        let core = spawn_core();
        core.request(CoreCommand::SetAudioConfig {
            config: AudioConfig {
                backend: AudioBackend::None,
                ..AudioConfig::default()
            },
        })
        .expect("set test audio backend");
        core
    }

    fn snapshot(core: &CoreHandle) -> StateSnapshot {
        match core
            .request(CoreCommand::GetState)
            .expect("request current core state")
            .as_slice()
        {
            [CoreEvent::StateSnapshot { state }] => state.clone(),
            events => panic!("unexpected state response: {events:?}"),
        }
    }

    fn shutdown(core: &CoreHandle) {
        core.request(CoreCommand::Shutdown)
            .expect("shutdown core after test");
    }

    fn large_test_midi() -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "meridian-cancel-load-{}-{}.mid",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock should be after unix epoch")
                .as_nanos()
        ));
        let mut track = Vec::with_capacity(80_000);
        for index in 0..40_000_u64 {
            let key = 21 + (index % 60) as u8;
            track.push(note_on(1, 0, key, 100));
            track.push(note_off(1, 0, key));
        }
        write_toolkit_midi(&path, 480, &[track]);
        path
    }

    #[test]
    fn loading_audio_for_a_new_midi_clears_the_old_display_context() {
        let core = test_core();
        let preview_midi = midi_fixture("smoke-two-notes.mid");
        let audio_only_midi = midi_fixture("piano/mozart-kv457-sonata-no14-fragment.mid");

        core.request(CoreCommand::LoadMidi {
            path: preview_midi.clone(),
        })
        .expect("load preview midi");
        let preview_state = snapshot(&core);
        assert!(preview_state.active_display_cache_id.is_some());
        assert!(preview_state.active_audio_cache_id.is_some());

        core.request(CoreCommand::LoadAudioMidi {
            path: audio_only_midi.clone(),
        })
        .expect("load audio-only midi");
        let replaced_state = snapshot(&core);

        assert_eq!(replaced_state.midi_path, Some(audio_only_midi));
        assert!(replaced_state.active_audio_cache_id.is_some());
        assert!(replaced_state.active_display_cache_id.is_none());
        assert!(!replaced_state.midi_loaded);

        shutdown(&core);
    }

    #[test]
    fn loading_display_for_the_same_midi_keeps_the_existing_audio_context() {
        let core = test_core();
        let midi = midi_fixture("smoke-two-notes.mid");

        core.request(CoreCommand::LoadAudioMidi { path: midi.clone() })
            .expect("load audio-only midi");
        let audio_state = snapshot(&core);
        assert!(audio_state.active_audio_cache_id.is_some());
        assert!(audio_state.active_display_cache_id.is_none());

        core.request(CoreCommand::LoadDisplayMidi { path: midi.clone() })
            .expect("load display midi for same file");
        let combined_state = snapshot(&core);

        assert_eq!(combined_state.midi_path, Some(midi));
        assert!(combined_state.active_audio_cache_id.is_some());
        assert!(combined_state.active_display_cache_id.is_some());
        assert!(combined_state.midi_loaded);

        shutdown(&core);
    }

    #[test]
    fn legacy_load_replaces_every_active_context_with_the_new_midi() {
        let core = test_core();
        let first_midi = midi_fixture("smoke-two-notes.mid");
        let second_midi = midi_fixture("piano/burgmuller-op100-no13-consolation.mid");

        core.request(CoreCommand::LoadAudioMidi {
            path: first_midi.clone(),
        })
        .expect("load first audio-only midi");
        let first_state = snapshot(&core);
        assert!(first_state.active_audio_cache_id.is_some());
        assert!(first_state.active_display_cache_id.is_none());

        core.request(CoreCommand::LoadMidi {
            path: second_midi.clone(),
        })
        .expect("load second preview midi");
        let replaced_state = snapshot(&core);

        assert_eq!(replaced_state.midi_path, Some(second_midi));
        assert!(replaced_state.active_audio_cache_id.is_some());
        assert!(replaced_state.active_display_cache_id.is_some());
        assert!(replaced_state.midi_loaded);

        shutdown(&core);
    }

    #[test]
    fn cancelling_a_midi_load_aborts_the_core_request() {
        let core = test_core();
        let receiver = core.subscribe_events();
        let midi = large_test_midi();
        let request_core = core.clone();
        let midi_for_thread = midi.clone();
        let load_thread = thread::spawn(move || {
            request_core
                .request(CoreCommand::LoadMidi {
                    path: midi_for_thread,
                })
                .expect("load midi request should return a response")
        });

        loop {
            let event = receiver.recv().expect("midi load progress event");
            if matches!(event, CoreEvent::MidiLoadProgress { .. }) {
                core.cancel_midi_loads();
                break;
            }
        }

        let response = load_thread.join().expect("load thread should join");
        assert!(matches!(
            response.as_slice(),
            [CoreEvent::Error {
                code: CoreErrorCode::Cancelled,
                ..
            }]
        ));

        let state = snapshot(&core);
        assert!(state.active_parsed_midi_id.is_none());
        assert!(state.active_display_cache_id.is_none());
        assert!(state.active_audio_cache_id.is_none());
        assert_eq!(state.midi_path, None);

        let _ = std::fs::remove_file(midi);
        shutdown(&core);
    }

    #[test]
    fn shutdown_waits_for_active_video_render_cancellation() {
        let core = test_core();
        let receiver = core.subscribe_events();
        let midi = large_test_midi();
        let output = std::env::temp_dir().join(format!(
            "meridian-shutdown-render-{}-{}.mp4",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock should be after unix epoch")
                .as_nanos()
        ));

        let start_events = core
            .request(CoreCommand::StartRenderVideo {
                config: crate::protocol::VideoRenderConfig {
                    midi_path: Some(midi.clone()),
                    output: output.clone(),
                    container: crate::protocol::VideoOutputContainer::Mp4,
                    fps: 4.0,
                    width: 160,
                    height: 90,
                    scene: Some(crate::render::SceneLayout::default().scene),
                    view_range: Some(2.0),
                    time_space: Some(DisplayTimeSpace::Tick),
                    first_key: None,
                    last_key: None,
                    ffmpeg_args: vec!["-y".to_string()],
                    export: Default::default(),
                    audio: None,
                },
            })
            .expect("start video render before shutdown");
        assert!(matches!(
            start_events.as_slice(),
            [CoreEvent::VideoRenderStatus {
                status: crate::protocol::VideoRenderStatus::Running { .. }
            }]
        ));

        let shutdown_core = core.clone();
        let shutdown_thread = thread::spawn(move || {
            shutdown_core
                .request(CoreCommand::Shutdown)
                .expect("shutdown request should succeed")
        });

        let mut saw_cancelled = false;
        for _ in 0..120 {
            match receiver.recv_timeout(Duration::from_secs(1)) {
                Ok(CoreEvent::VideoRender {
                    event: crate::protocol::VideoRenderEvent::RenderCancelled { .. },
                }) => {
                    saw_cancelled = true;
                    break;
                }
                Ok(CoreEvent::VideoRender {
                    event: crate::protocol::VideoRenderEvent::RenderFailed { message },
                }) => panic!("render should cancel during shutdown, got failure: {message}"),
                Ok(_) => {}
                Err(error) => panic!("timed out waiting for shutdown render cancellation: {error}"),
            }
        }

        assert!(saw_cancelled, "video render did not cancel before shutdown completed");

        let shutdown_response = shutdown_thread
            .join()
            .expect("shutdown thread should join");
        assert!(matches!(
            shutdown_response.as_slice(),
            [CoreEvent::ShutdownComplete]
        ));
        assert!(
            core.request(CoreCommand::GetState).is_err(),
            "core should stop receiving requests after shutdown completes"
        );

        let _ = std::fs::remove_file(midi);
        let _ = std::fs::remove_file(output);
    }
}
