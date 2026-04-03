mod analysis_job;
mod audio_render_job;
mod core_state;
mod midi_process_job;
mod resource_types;
mod resources;
mod state_ops;
mod support;
mod video_render_job;

use std::{
    sync::{Arc, Mutex},
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
}

pub fn spawn_core() -> CoreHandle {
    let (sender, receiver) = flume::unbounded();
    let subscribers = Arc::new(Mutex::new(Vec::new()));
    let core_handle = CoreHandle {
        sender: sender.clone(),
        subscribers: Arc::clone(&subscribers),
    };
    thread::spawn(move || {
        let mut core = core_state::CoreState::new(sender, subscribers);
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
    use std::path::PathBuf;

    use crate::{
        audio::{AudioBackend, AudioConfig},
        protocol::{CoreCommand, CoreEvent, StateSnapshot},
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
}
