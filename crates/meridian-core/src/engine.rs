use std::{path::PathBuf, thread, time::Instant};

use flume::{Receiver, Sender};

use crate::{
    error::MeridianError,
    midi::backend::{MIDIFileBase, MIDIFileUnion},
    protocol::{
        CoreCommand, CoreErrorCode, CoreEvent, FrameStats, ImageOutputFormat, RenderedFrame,
        StateSnapshot,
    },
    render::{SceneLayout, pfa::wgpu::save_scene_headless, project_scene},
};

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
}

pub type CoreResponse = Vec<CoreEvent>;

#[derive(Clone)]
pub struct CoreHandle {
    sender: Sender<RequestMessage>,
}

pub fn spawn_core() -> CoreHandle {
    let (sender, receiver) = flume::unbounded();
    thread::spawn(move || {
        let mut core = CoreState::default();
        core.run(receiver);
    });
    CoreHandle { sender }
}

impl CoreHandle {
    pub fn request(&self, command: CoreCommand) -> Result<CoreResponse, MeridianError> {
        let (reply_tx, reply_rx) = flume::bounded(1);
        self.sender
            .send(RequestMessage::Command {
                command,
                reply: reply_tx,
            })
            .map_err(|_| MeridianError::Wgpu("core request channel closed".into()))?;
        reply_rx
            .recv()
            .map_err(|_| MeridianError::Wgpu("core reply channel closed".into()))
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
            .map_err(|_| MeridianError::Wgpu("core request channel closed".into()))?;
        reply_rx
            .recv()
            .map_err(|_| MeridianError::Wgpu("core reply channel closed".into()))?
    }
}

#[derive(Default)]
struct CoreState {
    midi: Option<MIDIFileUnion>,
    midi_path: Option<PathBuf>,
    layout: SceneLayout,
    current_time: f64,
    playing: bool,
    last_tick: Option<Instant>,
}

impl CoreState {
    fn run(&mut self, receiver: Receiver<RequestMessage>) {
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
            CoreCommand::SetLayout {
                renderer,
                view_range,
                first_key,
                last_key,
                viewport_width,
                viewport_height,
            } => {
                if let Some(renderer) = renderer {
                    self.layout.renderer = renderer;
                }
                if let Some(view_range) = view_range {
                    self.layout.view_range = view_range.clamp(1.0, 30.0);
                }
                if let Some(first_key) = first_key {
                    self.layout.first_key = first_key;
                }
                if let Some(last_key) = last_key {
                    self.layout.last_key = last_key;
                }

                if let Err(error) = self.apply_viewport_overrides(viewport_width, viewport_height) {
                    return vec![error_event(
                        CoreErrorCode::InvalidViewport,
                        error.to_string(),
                    )];
                }

                match self.validate_layout() {
                    Ok(()) => vec![CoreEvent::StateSnapshot {
                        state: self.snapshot(),
                    }],
                    Err(event) => vec![event],
                }
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

    fn render_frame(
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
            layout: self.layout,
            stats,
            scene,
        })
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
            renderer: self.layout.renderer,
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
        Ok(())
    }
}

impl ImageOutputFormat {
    pub fn infer_from_path(path: &std::path::Path) -> Self {
        match path.extension().and_then(|ext| ext.to_str()) {
            Some("png") => Self::Png,
            Some("rgba") | Some("raw") => Self::Rgba,
            Some("ppm") | _ => Self::Ppm,
        }
    }
}

fn error_event(code: CoreErrorCode, message: impl Into<String>) -> CoreEvent {
    CoreEvent::Error {
        code,
        message: message.into(),
    }
}

fn event_to_error(context: &str, event: &CoreEvent) -> MeridianError {
    match event {
        CoreEvent::Error { message, .. } => {
            MeridianError::InvalidMidi(format!("{context}: {message}"))
        }
        _ => MeridianError::InvalidMidi(context.into()),
    }
}

fn error_code(error: &MeridianError) -> CoreErrorCode {
    match error {
        MeridianError::InvalidMidi(message) if message.contains("no midi loaded") => {
            CoreErrorCode::NoMidiLoaded
        }
        MeridianError::InvalidMidi(message)
            if message.contains("viewport_width") || message.contains("viewport_height") =>
        {
            CoreErrorCode::InvalidViewport
        }
        MeridianError::InvalidMidi(_) => CoreErrorCode::InvalidCommand,
        MeridianError::Io(_) | MeridianError::Platform(_) | MeridianError::SlintNotifier(_) => {
            CoreErrorCode::Internal
        }
        MeridianError::MidiLoad(_) => CoreErrorCode::InvalidCommand,
        MeridianError::Wgpu(_) => CoreErrorCode::Internal,
    }
}
