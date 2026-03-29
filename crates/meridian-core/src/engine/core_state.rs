use std::{
    path::PathBuf,
    sync::{Arc, Mutex, atomic::AtomicBool},
    time::Instant,
};

use flume::{Receiver, Sender};

use crate::{
    midi::backend::MIDIFileUnion,
    protocol::{CoreCommand, CoreErrorCode, CoreEvent, VideoRenderStatus},
    render::SceneLayout,
};

use super::{CoreHandle, CoreResponse, RequestMessage, support::error_event};

pub(super) struct RenderJobState {
    pub(super) cancel: Arc<AtomicBool>,
    pub(super) status: VideoRenderStatus,
}

pub(super) struct CoreState {
    pub(super) core_handle: CoreHandle,
    pub(super) subscribers: Arc<Mutex<Vec<Sender<CoreEvent>>>>,
    pub(super) midi: Option<MIDIFileUnion>,
    pub(super) midi_path: Option<PathBuf>,
    pub(super) layout: SceneLayout,
    pub(super) scene_physics: crate::render::ScenePhysicsState,
    pub(super) current_time: f64,
    pub(super) playing: bool,
    pub(super) last_tick: Option<Instant>,
    pub(super) last_physics_tick: Option<Instant>,
    pub(super) render_job: Option<RenderJobState>,
}

impl CoreState {
    pub(super) fn new(
        sender: Sender<RequestMessage>,
        subscribers: Arc<Mutex<Vec<Sender<CoreEvent>>>>,
    ) -> Self {
        Self {
            core_handle: CoreHandle {
                sender,
                subscribers: Arc::clone(&subscribers),
            },
            subscribers,
            midi: None,
            midi_path: None,
            layout: SceneLayout::default(),
            scene_physics: crate::render::ScenePhysicsState::new(&SceneLayout::default().scene),
            current_time: 0.0,
            playing: false,
            last_tick: None,
            last_physics_tick: None,
            render_job: None,
        }
    }

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
                RequestMessage::VideoRenderUpdate { event } => {
                    self.handle_video_render_update(event);
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
                    if let Err(error) = self.refresh_note_colors() {
                        return vec![error_event(CoreErrorCode::Internal, error.to_string())];
                    }
                    self.scene_physics.reset(&self.layout.scene);
                    self.current_time = 0.0;
                    let now = Instant::now();
                    self.last_tick = Some(now);
                    self.last_physics_tick = Some(now);
                    vec![CoreEvent::MidiLoaded {
                        path,
                        state: self.snapshot(),
                    }]
                }
                Err(error) => vec![error_event(CoreErrorCode::Internal, error.to_string())],
            },
            CoreCommand::SetTime { time } => {
                self.current_time = time.clamp(0.0, self.midi_length().max(0.0));
                self.last_physics_tick = Some(Instant::now());
                vec![CoreEvent::StateSnapshot {
                    state: self.snapshot(),
                }]
            }
            CoreCommand::TickProjectorPhysics { delta_seconds } => {
                if let Err(error) = self.tick_projector_physics(delta_seconds) {
                    return vec![error_event(CoreErrorCode::Internal, error.to_string())];
                }
                self.last_physics_tick = Some(Instant::now());
                vec![CoreEvent::StateSnapshot {
                    state: self.snapshot(),
                }]
            }
            CoreCommand::ResetProjectorPhysics => {
                self.scene_physics.reset(&self.layout.scene);
                self.last_physics_tick = Some(Instant::now());
                vec![CoreEvent::StateSnapshot {
                    state: self.snapshot(),
                }]
            }
            CoreCommand::StepTime { delta } => {
                self.current_time =
                    (self.current_time + delta).clamp(0.0, self.midi_length().max(0.0));
                self.last_physics_tick = Some(Instant::now());
                vec![CoreEvent::StateSnapshot {
                    state: self.snapshot(),
                }]
            }
            CoreCommand::SetPlaying { playing } => {
                self.playing = playing;
                let now = Instant::now();
                self.last_tick = Some(now);
                self.last_physics_tick = Some(now);
                vec![CoreEvent::StateSnapshot {
                    state: self.snapshot(),
                }]
            }
            CoreCommand::TogglePlaying => {
                self.playing = !self.playing;
                let now = Instant::now();
                self.last_tick = Some(now);
                self.last_physics_tick = Some(now);
                vec![CoreEvent::StateSnapshot {
                    state: self.snapshot(),
                }]
            }
            CoreCommand::SetSceneConfig { scene } => {
                self.layout.scene = scene;
                self.scene_physics.reset(&self.layout.scene);
                if let Err(error) = self.refresh_note_colors() {
                    return vec![error_event(CoreErrorCode::Internal, error.to_string())];
                }
                self.last_physics_tick = Some(Instant::now());
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
                Err(error) => vec![error_event(super::error_code(&error), error.to_string())],
            },
            CoreCommand::SaveFrame {
                output,
                format,
                viewport_width,
                viewport_height,
            } => {
                let format = format.unwrap_or_else(|| {
                    crate::protocol::ImageOutputFormat::infer_from_path(&output)
                });
                self.save_frame_headless(output, format, viewport_width, viewport_height)
            }
            CoreCommand::StartRenderVideo { config } => self.start_render_video(config),
            CoreCommand::CancelRenderVideo => self.cancel_render_video(),
            CoreCommand::GetRenderVideoStatus => vec![CoreEvent::VideoRenderStatus {
                status: self.video_render_status(),
            }],
            CoreCommand::Shutdown => vec![CoreEvent::ShutdownComplete],
        }
    }
}
