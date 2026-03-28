use std::{path::PathBuf, time::Instant};

use flume::Receiver;

use crate::{
    error::MeridianError,
    midi::backend::{MIDIFileBase, MIDIFileUnion},
    protocol::{
        CoreCommand, CoreErrorCode, CoreEvent, FrameStats, ImageOutputFormat, RenderedFrame,
        StateSnapshot,
    },
    render::{SceneConfig, SceneLayout, pfa::wgpu::save_scene_headless, project_scene},
};

use super::{
    CoreResponse, RequestMessage,
    support::{error_code, error_event, event_to_error},
};

#[derive(Default)]
pub(super) struct CoreState {
    midi: Option<MIDIFileUnion>,
    midi_path: Option<PathBuf>,
    layout: SceneLayout,
    current_time: f64,
    playing: bool,
    last_tick: Option<Instant>,
}

impl CoreState {
    pub(super) fn run(&mut self, receiver: Receiver<RequestMessage>) {
        for request in receiver {
            match request {
                RequestMessage::Command { command, reply } => {
                    let should_shutdown = matches!(command, CoreCommand::Shutdown);
                    let response = self.handle(command);
                    let _ = reply.send(response);
                    if should_shutdown {
                        break;
                    }
                }
                RequestMessage::RenderFrame {
                    viewport_width,
                    viewport_height,
                    reply,
                } => {
                    let _ = reply.send(self.render_frame(viewport_width, viewport_height));
                }
            }
        }
    }

    fn handle(&mut self, command: CoreCommand) -> CoreResponse {
        if !matches!(command, CoreCommand::Shutdown) {
            self.sync_time();
        }

        match command {
            CoreCommand::GetState => vec![CoreEvent::StateSnapshot {
                state: self.snapshot(),
            }],
            CoreCommand::LoadMidi { path } => match MIDIFileUnion::load_ram(&path) {
                Ok(midi) => {
                    self.midi_path = Some(path.clone());
                    self.midi = Some(midi);
                    self.current_time = 0.0;
                    vec![CoreEvent::MidiLoaded {
                        path,
                        state: self.snapshot(),
                    }]
                }
                Err(error) => vec![error_event(CoreErrorCode::Internal, error.to_string())],
            },
            CoreCommand::SetTime { time } => {
                self.current_time = time.clamp(0.0, self.midi_length().max(0.0));
                vec![CoreEvent::StateSnapshot {
                    state: self.snapshot(),
                }]
            }
            CoreCommand::StepTime { delta } => {
                self.current_time =
                    (self.current_time + delta).clamp(0.0, self.midi_length().max(0.0));
                vec![CoreEvent::StateSnapshot {
                    state: self.snapshot(),
                }]
            }
            CoreCommand::SetPlaying { playing } => {
                self.playing = playing;
                self.last_tick = Some(Instant::now());
                vec![CoreEvent::StateSnapshot {
                    state: self.snapshot(),
                }]
            }
            CoreCommand::TogglePlaying => {
                self.playing = !self.playing;
                self.last_tick = Some(Instant::now());
                vec![CoreEvent::StateSnapshot {
                    state: self.snapshot(),
                }]
            }
            CoreCommand::SetSceneConfig { scene } => {
                self.layout.scene = scene;
                self.snapshot_after_layout_validation()
            }
            CoreCommand::SetViewRange { seconds } => {
                self.layout.view_range = seconds.clamp(1.0, 30.0);
                vec![CoreEvent::StateSnapshot {
                    state: self.snapshot(),
                }]
            }
            CoreCommand::SetKeyRange {
                first_key,
                last_key,
            } => {
                self.layout.first_key = first_key;
                self.layout.last_key = last_key;
                self.snapshot_after_layout_validation()
            }
            CoreCommand::SetViewport { width, height } => {
                if let Err(error) = self.apply_viewport_overrides(Some(width), Some(height)) {
                    return vec![error_event(
                        CoreErrorCode::InvalidViewport,
                        error.to_string(),
                    )];
                }
                self.snapshot_after_layout_validation()
            }
            CoreCommand::RenderFrame {
                viewport_width,
                viewport_height,
            } => match self.render_frame(viewport_width, viewport_height) {
                Ok(frame) => vec![CoreEvent::FrameProjected {
                    state: frame.state,
                    layout: frame.layout,
                    stats: frame.stats,
                }],
                Err(error) => vec![error_event(error_code(&error), error.to_string())],
            },
            CoreCommand::SaveFrame {
                output,
                format,
                viewport_width,
                viewport_height,
            } => match self.render_frame(viewport_width, viewport_height) {
                Ok(frame) => {
                    let format =
                        format.unwrap_or_else(|| ImageOutputFormat::infer_from_path(&output));
                    match save_scene_headless(
                        frame.layout.viewport_width,
                        frame.layout.viewport_height,
                        &frame.scene,
                        format,
                        &output,
                    ) {
                        Ok(bytes_written) => vec![CoreEvent::FrameSaved {
                            output,
                            format,
                            state: frame.state,
                            stats: frame.stats,
                            bytes_written,
                        }],
                        Err(error) => vec![error_event(error_code(&error), error.to_string())],
                    }
                }
                Err(error) => vec![error_event(error_code(&error), error.to_string())],
            },
            CoreCommand::Shutdown => vec![CoreEvent::ShutdownComplete],
        }
    }

    pub(super) fn render_frame(
        &mut self,
        viewport_width: Option<u32>,
        viewport_height: Option<u32>,
    ) -> Result<RenderedFrame, MeridianError> {
        self.sync_time();
        self.apply_viewport_overrides(viewport_width, viewport_height)?;
        self.validate_layout()
            .map_err(|event| event_to_error("invalid layout", &event))?;
        let scene = self
            .project_current_scene()
            .map_err(MeridianError::InvalidMidi)?;
        let stats = FrameStats::from_scene(&scene);

        Ok(RenderedFrame {
            state: self.snapshot(),
            layout: self.layout.clone(),
            stats,
            scene,
        })
    }

    fn snapshot_after_layout_validation(&self) -> CoreResponse {
        match self.validate_layout() {
            Ok(()) => vec![CoreEvent::StateSnapshot {
                state: self.snapshot(),
            }],
            Err(event) => vec![event],
        }
    }

    fn apply_viewport_overrides(
        &mut self,
        viewport_width: Option<u32>,
        viewport_height: Option<u32>,
    ) -> Result<(), MeridianError> {
        if let Some(viewport_width) = viewport_width {
            if viewport_width == 0 {
                return Err(MeridianError::InvalidMidi(
                    "viewport_width must be > 0".into(),
                ));
            }
            self.layout.viewport_width = viewport_width;
        }
        if let Some(viewport_height) = viewport_height {
            if viewport_height == 0 {
                return Err(MeridianError::InvalidMidi(
                    "viewport_height must be > 0".into(),
                ));
            }
            self.layout.viewport_height = viewport_height;
        }
        Ok(())
    }

    fn project_current_scene(&mut self) -> Result<crate::render::ProjectedScene, String> {
        let midi = self
            .midi
            .as_mut()
            .ok_or_else(|| "no midi loaded".to_string())?;
        Ok(project_scene(midi, self.current_time, &self.layout))
    }

    fn midi_length(&self) -> f64 {
        self.midi
            .as_ref()
            .and_then(|midi| midi.midi_length())
            .unwrap_or(0.0)
    }

    fn total_notes(&self) -> u64 {
        self.midi
            .as_ref()
            .and_then(|midi| midi.stats().total_notes)
            .unwrap_or(0)
    }

    fn snapshot(&self) -> StateSnapshot {
        StateSnapshot {
            midi_path: self.midi_path.clone(),
            midi_loaded: self.midi.is_some(),
            scene: self.layout.scene.clone(),
            current_time: self.current_time,
            playing: self.playing,
            midi_length: self.midi_length(),
            total_notes: self.total_notes(),
            view_range: self.layout.view_range,
            first_key: self.layout.first_key,
            last_key: self.layout.last_key,
            viewport_width: self.layout.viewport_width,
            viewport_height: self.layout.viewport_height,
        }
    }

    fn sync_time(&mut self) {
        let now = Instant::now();
        if self.playing {
            if let Some(last_tick) = self.last_tick {
                self.current_time += now.duration_since(last_tick).as_secs_f64();
                self.current_time = self.current_time.min(self.midi_length().max(0.0));
                if self.current_time >= self.midi_length() && self.midi_length() > 0.0 {
                    self.playing = false;
                }
            }
        }
        self.last_tick = Some(now);
    }

    fn validate_layout(&self) -> Result<(), CoreEvent> {
        if self.layout.viewport_width == 0 || self.layout.viewport_height == 0 {
            return Err(error_event(
                CoreErrorCode::InvalidViewport,
                "viewport dimensions must be greater than zero",
            ));
        }
        if self.layout.first_key > self.layout.last_key {
            return Err(error_event(
                CoreErrorCode::InvalidLayout,
                "first_key must be <= last_key",
            ));
        }
        if matches!(self.layout.scene, SceneConfig::ThreeD(_)) {
            return Err(error_event(
                CoreErrorCode::InvalidLayout,
                "3d scene config is reserved but not implemented yet",
            ));
        }
        Ok(())
    }
}
