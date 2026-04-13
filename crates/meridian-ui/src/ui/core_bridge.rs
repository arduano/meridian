//! Core command bridge for the UI.
//!
//! This layer is deliberately narrow: it sends commands to `meridian-core`,
//! requests fresh snapshots, and feeds resulting events back through the shared
//! UI reducer. Feature-specific UI behavior should live in the runtime modules,
//! not here.

use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use meridian_core::{
    CoreHandle, MeridianError,
    audio::{AudioConfig, AudioRenderConfig},
    display::MIN_VIEW_RANGE_SECONDS,
    midi::{MidiFileProcessingConfig, analysis::MidiAnalysisKind},
    protocol::{CoreCommand, CoreEvent, MidiProcessStatus, ParsedMidiId, VideoRenderConfig},
    render::{DisplayTimeSpace, RendererKind, SceneConfig, SceneLayout},
    transport::PREVIEW_START_TIME_SECONDS,
};

use super::{
    state::{UiStartupOptions, reduce_core_events},
    view_model::{TransportViewModel, UiViewModel},
};

/// Thin adapter between Slint callbacks and `meridian-core`.
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

    pub fn cancel_midi_loads(&self) {
        self.core.cancel_midi_loads();
    }

    pub fn initialize(
        &self,
        options: &UiStartupOptions,
        model: &Arc<Mutex<UiViewModel>>,
    ) -> Result<(), MeridianError> {
        self.request(
            CoreCommand::SetAudioConfig {
                config: options.audio.clone(),
            },
            model,
        )?;
        self.request(
            CoreCommand::SetSceneConfig {
                scene: options.scene.clone(),
            },
            model,
        )?;
        self.request(
            Self::view_range_command(options.view_range, options.time_space),
            model,
        )?;
        self.request(
            CoreCommand::SetKeyRange {
                first_key: options.first_key,
                last_key: options.last_key,
            },
            model,
        )?;
        self.request(
            CoreCommand::SetViewport {
                width: 1280,
                height: 720,
            },
            model,
        )?;
        self.request(
            CoreCommand::SetTime {
                time: options.start_time.max(PREVIEW_START_TIME_SECONDS),
            },
            model,
        )?;
        if let Some(path) = &options.midi_path {
            self.load_midi(path.clone(), model)?;
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

    pub fn drop_inactive_midi_resources(
        &self,
        model: &Arc<Mutex<UiViewModel>>,
    ) -> Result<Vec<CoreEvent>, MeridianError> {
        self.request(CoreCommand::DropInactiveMidiResources, model)
    }

    pub fn load_parsed_midi(
        &self,
        path: PathBuf,
        model: &Arc<Mutex<UiViewModel>>,
    ) -> Result<Vec<CoreEvent>, MeridianError> {
        self.request(CoreCommand::LoadParsedMidi { path }, model)
    }

    pub fn start_midi_analysis_job(
        &self,
        parsed_midi_id: ParsedMidiId,
        kinds: Vec<MidiAnalysisKind>,
        bucket_count: Option<usize>,
        model: &Arc<Mutex<UiViewModel>>,
    ) -> Result<Vec<CoreEvent>, MeridianError> {
        self.request(
            CoreCommand::StartMidiAnalysisJob {
                parsed_midi_id,
                kinds,
                bucket_count,
            },
            model,
        )
    }
    pub fn start_process_midi_file(
        &self,
        input: PathBuf,
        output: PathBuf,
        config: MidiFileProcessingConfig,
        model: &Arc<Mutex<UiViewModel>>,
    ) -> Result<Vec<CoreEvent>, MeridianError> {
        self.request(
            CoreCommand::StartProcessMidiFile {
                input,
                output,
                config,
            },
            model,
        )
    }

    pub fn cancel_midi_file_process(
        &self,
        model: &Arc<Mutex<UiViewModel>>,
    ) -> Result<Vec<CoreEvent>, MeridianError> {
        self.request(CoreCommand::CancelMidiFileProcess, model)
    }

    pub fn get_midi_file_process_status(
        &self,
        model: &Arc<Mutex<UiViewModel>>,
    ) -> Result<MidiProcessStatus, MeridianError> {
        let events = self.request(CoreCommand::GetMidiFileProcessStatus, model)?;
        events
            .into_iter()
            .find_map(|event| match event {
                CoreEvent::MidiProcessStatus { status } => Some(status),
                _ => None,
            })
            .ok_or_else(|| MeridianError::Protocol("missing midi process status".into()))
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
        let transport = self.current_transport(model);
        let zoom_factor = std::f64::consts::SQRT_2.powf(delta);
        self.request(
            Self::view_range_command(
                (transport.view_range * zoom_factor).max(MIN_VIEW_RANGE_SECONDS),
                transport.time_space,
            ),
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
        self.request(
            CoreCommand::SetTime {
                time: time.max(PREVIEW_START_TIME_SECONDS),
            },
            model,
        )
    }

    pub fn set_time_space(
        &self,
        time_space: DisplayTimeSpace,
        model: &Arc<Mutex<UiViewModel>>,
    ) -> Result<Vec<CoreEvent>, MeridianError> {
        let current = self.current_transport(model);
        self.request(
            Self::view_range_command(current.view_range.max(MIN_VIEW_RANGE_SECONDS), time_space),
            model,
        )
    }

    pub fn set_renderer(
        &self,
        renderer: RendererKind,
        model: &Arc<Mutex<UiViewModel>>,
    ) -> Result<Vec<CoreEvent>, MeridianError> {
        let scene = self.current_scene(model).unwrap_or_default();
        self.request(
            CoreCommand::SetSceneConfig {
                scene: Self::scene_with_renderer(scene, renderer),
            },
            model,
        )
    }

    pub fn set_view_range_value(
        &self,
        seconds: f64,
        model: &Arc<Mutex<UiViewModel>>,
    ) -> Result<Vec<CoreEvent>, MeridianError> {
        let current = self.current_transport(model);
        self.request(Self::view_range_command(seconds, current.time_space), model)
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
        let Some(mut scene) = self.current_scene(model) else {
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

    pub fn start_render_audio(
        &self,
        config: AudioRenderConfig,
        model: &Arc<Mutex<UiViewModel>>,
    ) -> Result<Vec<CoreEvent>, MeridianError> {
        self.request(CoreCommand::StartRenderAudio { config }, model)
    }

    pub fn cancel_render_audio(
        &self,
        model: &Arc<Mutex<UiViewModel>>,
    ) -> Result<Vec<CoreEvent>, MeridianError> {
        self.request(CoreCommand::CancelRenderAudio, model)
    }

    pub fn start_render_video(
        &self,
        config: VideoRenderConfig,
        model: &Arc<Mutex<UiViewModel>>,
    ) -> Result<Vec<CoreEvent>, MeridianError> {
        self.request(CoreCommand::StartRenderVideo { config }, model)
    }

    pub fn cancel_render_video(
        &self,
        model: &Arc<Mutex<UiViewModel>>,
    ) -> Result<Vec<CoreEvent>, MeridianError> {
        self.request(CoreCommand::CancelRenderVideo, model)
    }

    fn request(
        &self,
        command: CoreCommand,
        model: &Arc<Mutex<UiViewModel>>,
    ) -> Result<Vec<CoreEvent>, MeridianError> {
        // Keep bridge callers thin: every command is reduced into the shared UI model here.
        let events = self.core.request(command)?;
        reduce_core_events(model, &events);
        Ok(events)
    }

    fn current_transport(&self, model: &Arc<Mutex<UiViewModel>>) -> TransportViewModel {
        model
            .lock()
            .expect("ui model mutex poisoned")
            .transport
            .clone()
    }

    fn current_scene(&self, model: &Arc<Mutex<UiViewModel>>) -> Option<SceneConfig> {
        model
            .lock()
            .expect("ui model mutex poisoned")
            .snapshot
            .as_ref()
            .map(|snapshot| snapshot.scene.clone())
    }

    fn view_range_command(seconds: f64, time_space: DisplayTimeSpace) -> CoreCommand {
        CoreCommand::SetViewRange {
            seconds,
            time_space: Some(time_space),
        }
    }

    fn scene_with_renderer(scene: SceneConfig, renderer: RendererKind) -> SceneConfig {
        let mut layout = SceneLayout {
            scene,
            ..SceneLayout::default()
        };
        layout.set_renderer_kind(renderer);
        layout.scene
    }
}
