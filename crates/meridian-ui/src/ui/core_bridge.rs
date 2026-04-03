use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use meridian_core::{
    CoreHandle, MeridianError,
    audio::{AudioBackend, AudioConfig},
    display::MIN_VIEW_RANGE_SECONDS,
    midi::MidiProcessingConfig,
    protocol::{CoreCommand, CoreEvent, ParsedMidiId, ProcessedMidiId},
    render::{DisplayTimeSpace, RendererKind, SceneConfig, SceneLayout},
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
                    time_space: None,
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

    pub fn load_display_midi(
        &self,
        path: PathBuf,
        model: &Arc<Mutex<UiViewModel>>,
    ) -> Result<Vec<CoreEvent>, MeridianError> {
        self.request(CoreCommand::LoadDisplayMidi { path }, model)
    }

    pub fn load_audio_midi(
        &self,
        path: PathBuf,
        model: &Arc<Mutex<UiViewModel>>,
    ) -> Result<Vec<CoreEvent>, MeridianError> {
        self.request(CoreCommand::LoadAudioMidi { path }, model)
    }

    pub fn unload_render_context(
        &self,
        model: &Arc<Mutex<UiViewModel>>,
    ) -> Result<Vec<CoreEvent>, MeridianError> {
        self.request(CoreCommand::UnloadRenderContext, model)
    }

    pub fn load_parsed_midi(
        &self,
        path: PathBuf,
        model: &Arc<Mutex<UiViewModel>>,
    ) -> Result<Vec<CoreEvent>, MeridianError> {
        self.request(CoreCommand::LoadParsedMidi { path }, model)
    }

    pub fn build_processed_midi(
        &self,
        parsed_midi_id: ParsedMidiId,
        config: MidiProcessingConfig,
        model: &Arc<Mutex<UiViewModel>>,
    ) -> Result<Vec<CoreEvent>, MeridianError> {
        self.request(
            CoreCommand::BuildProcessedMidi {
                parsed_midi_id,
                config,
            },
            model,
        )
    }

    pub fn analyze_processed_midi(
        &self,
        processed_midi_id: ProcessedMidiId,
        bucket_count: Option<usize>,
        model: &Arc<Mutex<UiViewModel>>,
    ) -> Result<Vec<CoreEvent>, MeridianError> {
        self.request(
            CoreCommand::AnalyzeProcessedMidi {
                processed_midi_id,
                bucket_count,
            },
            model,
        )
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
        let transport = model
            .lock()
            .expect("ui model mutex poisoned")
            .transport
            .clone();
        let zoom_factor = std::f64::consts::SQRT_2.powf(delta);
        self.request(
            CoreCommand::SetViewRange {
                seconds: (transport.view_range * zoom_factor).max(MIN_VIEW_RANGE_SECONDS),
                time_space: Some(transport.time_space),
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

    pub fn set_playing(
        &self,
        playing: bool,
        model: &Arc<Mutex<UiViewModel>>,
    ) -> Result<Vec<CoreEvent>, MeridianError> {
        self.request(CoreCommand::SetPlaying { playing }, model)
    }

    pub fn set_audio_config(
        &self,
        config: AudioConfig,
        model: &Arc<Mutex<UiViewModel>>,
    ) -> Result<Vec<CoreEvent>, MeridianError> {
        self.request(CoreCommand::SetAudioConfig { config }, model)
    }

    pub fn seek_time(
        &self,
        time: f64,
        model: &Arc<Mutex<UiViewModel>>,
    ) -> Result<Vec<CoreEvent>, MeridianError> {
        self.request(CoreCommand::SetTime { time }, model)
    }

    pub fn set_time_space(
        &self,
        time_space: DisplayTimeSpace,
        model: &Arc<Mutex<UiViewModel>>,
    ) -> Result<Vec<CoreEvent>, MeridianError> {
        let current = model
            .lock()
            .expect("ui model mutex poisoned")
            .transport
            .clone();
        self.request(
            CoreCommand::SetViewRange {
                seconds: current.view_range.max(MIN_VIEW_RANGE_SECONDS),
                time_space: Some(time_space),
            },
            model,
        )
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

    pub fn set_view_range_value(
        &self,
        seconds: f64,
        model: &Arc<Mutex<UiViewModel>>,
    ) -> Result<Vec<CoreEvent>, MeridianError> {
        let current = model
            .lock()
            .expect("ui model mutex poisoned")
            .transport
            .clone();
        self.request(
            CoreCommand::SetViewRange {
                seconds,
                time_space: Some(current.time_space),
            },
            model,
        )
    }

    pub fn set_key_range(
        &self,
        first_key: u8,
        last_key: u8,
        model: &Arc<Mutex<UiViewModel>>,
    ) -> Result<Vec<CoreEvent>, MeridianError> {
        self.request(
            CoreCommand::SetKeyRange {
                first_key,
                last_key,
            },
            model,
        )
    }

    pub fn update_scene(
        &self,
        model: &Arc<Mutex<UiViewModel>>,
        mutate: impl FnOnce(&mut SceneConfig),
    ) -> Result<Vec<CoreEvent>, MeridianError> {
        let Some(mut scene) = model
            .lock()
            .expect("ui model mutex poisoned")
            .snapshot
            .as_ref()
            .map(|snapshot| snapshot.scene.clone())
        else {
            return Ok(Vec::new());
        };
        mutate(&mut scene);
        self.request(CoreCommand::SetSceneConfig { scene }, model)
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
