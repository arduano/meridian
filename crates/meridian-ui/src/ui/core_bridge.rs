use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use meridian_core::{
    CoreHandle, MeridianError,
    audio::{AudioBackend, AudioConfig},
    protocol::{CoreCommand, CoreEvent},
    render::{RendererKind, SceneLayout},
};

use super::{state::UiOptions, view_model::UiViewModel};

#[derive(Clone)]
pub struct UiCoreBridge {
    core: CoreHandle,
}

impl UiCoreBridge {
    pub fn new(core: CoreHandle) -> Self {
        Self { core }
    }

    pub fn core(&self) -> &CoreHandle {
        &self.core
    }

    pub fn initialize(
        &self,
        options: &UiOptions,
        model: &Arc<Mutex<UiViewModel>>,
    ) -> Result<(), MeridianError> {
        let mut layout = SceneLayout::default();
        layout.set_renderer_kind(options.renderer);
        for events in [
            self.request(
                CoreCommand::SetAudioConfig {
                    config: AudioConfig {
                        backend: AudioBackend::Xsynth,
                        ..AudioConfig::default()
                    },
                },
                model,
            )?,
            self.request(
                CoreCommand::SetSceneConfig {
                    scene: layout.scene.clone(),
                },
                model,
            )?,
            self.request(
                CoreCommand::SetViewRange {
                    seconds: options.view_range,
                },
                model,
            )?,
            self.request(
                CoreCommand::SetKeyRange {
                    first_key: options.first_key,
                    last_key: options.last_key,
                },
                model,
            )?,
            self.request(
                CoreCommand::SetViewport {
                    width: 1280,
                    height: 720,
                },
                model,
            )?,
            self.request(
                CoreCommand::SetTime {
                    time: options.start_time.max(0.0),
                },
                model,
            )?,
        ] {
            let _ = events;
        }
        if let Some(path) = &options.midi_path {
            let _ = self.load_midi(path.clone(), model)?;
        }
        Ok(())
    }

    pub fn load_midi(
        &self,
        path: PathBuf,
        model: &Arc<Mutex<UiViewModel>>,
    ) -> Result<Vec<CoreEvent>, MeridianError> {
        self.request(CoreCommand::LoadMidi { path }, model)
    }

    pub fn step_time(
        &self,
        delta: f64,
        model: &Arc<Mutex<UiViewModel>>,
    ) -> Result<Vec<CoreEvent>, MeridianError> {
        self.request(CoreCommand::StepTime { delta }, model)
    }

    pub fn zoom(
        &self,
        delta: f64,
        model: &Arc<Mutex<UiViewModel>>,
    ) -> Result<Vec<CoreEvent>, MeridianError> {
        let current = model
            .lock()
            .expect("ui model mutex poisoned")
            .transport
            .view_range;
        self.request(
            CoreCommand::SetViewRange {
                seconds: (current + delta).clamp(1.0, 30.0),
            },
            model,
        )
    }

    pub fn toggle_play(
        &self,
        model: &Arc<Mutex<UiViewModel>>,
    ) -> Result<Vec<CoreEvent>, MeridianError> {
        self.request(CoreCommand::TogglePlaying, model)
    }

    pub fn seek_time(
        &self,
        time: f64,
        model: &Arc<Mutex<UiViewModel>>,
    ) -> Result<Vec<CoreEvent>, MeridianError> {
        self.request(CoreCommand::SetTime { time }, model)
    }

    pub fn set_renderer(
        &self,
        renderer: RendererKind,
        model: &Arc<Mutex<UiViewModel>>,
    ) -> Result<Vec<CoreEvent>, MeridianError> {
        let mut layout = SceneLayout::default();
        layout.set_renderer_kind(renderer);
        self.request(
            CoreCommand::SetSceneConfig {
                scene: layout.scene,
            },
            model,
        )
    }

    pub fn refresh_state(
        &self,
        model: &Arc<Mutex<UiViewModel>>,
    ) -> Result<Vec<CoreEvent>, MeridianError> {
        self.request(CoreCommand::GetState, model)
    }

    fn request(
        &self,
        command: CoreCommand,
        model: &Arc<Mutex<UiViewModel>>,
    ) -> Result<Vec<CoreEvent>, MeridianError> {
        let events = self.core.request(command)?;
        model
            .lock()
            .expect("ui model mutex poisoned")
            .reduce_events(&events);
        Ok(events)
    }
}
