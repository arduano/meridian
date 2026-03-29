use std::{
    path::PathBuf,
    sync::{Arc, Mutex, atomic::AtomicBool},
    time::Instant,
};

use flume::{Receiver, Sender};

use crate::{
    audio::{AudioConfig, LiveAudioSession, MeridianAudioPlayer, PlaybackClock},
    display::LiveDisplaySession,
    midi::MidiCacheStack,
    protocol::{AudioRenderStatus, CoreCommand, CoreErrorCode, CoreEvent, VideoRenderStatus},
    transport::TransportState,
};

use super::{CoreHandle, CoreResponse, RequestMessage, support::error_event};

pub(super) struct RenderJobState {
    pub(super) cancel: Arc<AtomicBool>,
    pub(super) status: VideoRenderStatus,
}

pub(super) struct AudioRenderJobState {
    pub(super) cancel: Arc<AtomicBool>,
    pub(super) status: AudioRenderStatus,
}

pub(super) struct CoreState {
    pub(super) core_handle: CoreHandle,
    pub(super) subscribers: Arc<Mutex<Vec<Sender<CoreEvent>>>>,
    pub(super) midi_cache: Option<MidiCacheStack>,
    pub(super) display: LiveDisplaySession,
    pub(super) audio_config: AudioConfig,
    pub(super) audio_player: Arc<MeridianAudioPlayer>,
    pub(super) audio_clock: Arc<PlaybackClock>,
    pub(super) audio_session: Option<LiveAudioSession>,
    pub(super) midi_path: Option<PathBuf>,
    pub(super) transport: TransportState,
    pub(super) render_job: Option<RenderJobState>,
    pub(super) audio_render_job: Option<AudioRenderJobState>,
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
            midi_cache: None,
            display: LiveDisplaySession::new(),
            audio_config: AudioConfig::default(),
            audio_player: MeridianAudioPlayer::new(&AudioConfig::default()),
            audio_clock: Arc::new(PlaybackClock::new()),
            audio_session: None,
            midi_path: None,
            transport: TransportState::new(),
            render_job: None,
            audio_render_job: None,
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
                RequestMessage::AudioRenderUpdate { event } => {
                    self.handle_audio_render_update(event);
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
            CoreCommand::LoadMidi { path } => match MidiCacheStack::load(&path)
                .and_then(|cache| cache.instantiate_in_ram().map(|midi| (cache, midi)))
            {
                Ok((cache, midi)) => {
                    self.audio_session = None;
                    self.midi_cache = Some(cache);
                    self.midi_path = Some(path.clone());
                    self.display.load_midi(midi, Instant::now());
                    if let Err(error) = self.refresh_note_colors() {
                        return vec![error_event(CoreErrorCode::Internal, error.to_string())];
                    }
                    let now = Instant::now();
                    self.transport.reset(now);
                    self.display.mark_physics_tick(now);
                    self.audio_clock.set_time(0.0);
                    self.audio_clock.set_playing(false);
                    self.start_audio_session();
                    vec![CoreEvent::MidiLoaded {
                        path,
                        state: self.snapshot(),
                    }]
                }
                Err(error) => vec![error_event(CoreErrorCode::Internal, error.to_string())],
            },
            CoreCommand::SetAudioConfig { config } => {
                self.audio_config = config;
                if let Err(error) = self.audio_player.switch(&self.audio_config) {
                    return vec![error_event(CoreErrorCode::Internal, error.to_string())];
                }
                self.restart_audio_session();
                vec![
                    CoreEvent::StateSnapshot {
                        state: self.snapshot(),
                    },
                    CoreEvent::AudioStatus {
                        status: self.audio_player.status(),
                    },
                ]
            }
            CoreCommand::GetAudioStatus => vec![CoreEvent::AudioStatus {
                status: self.audio_player.status(),
            }],
            CoreCommand::StartRenderAudio { config } => self.start_render_audio(config),
            CoreCommand::CancelRenderAudio => self.cancel_render_audio(),
            CoreCommand::GetRenderAudioStatus => vec![CoreEvent::AudioRenderStatus {
                status: self.audio_render_status(),
            }],
            CoreCommand::SetTime { time } => {
                self.transport
                    .set_time(time, self.midi_length(), Instant::now());
                self.display.mark_physics_tick(Instant::now());
                self.audio_clock.set_time(self.transport.current_time());
                vec![CoreEvent::StateSnapshot {
                    state: self.snapshot(),
                }]
            }
            CoreCommand::TickProjectorPhysics { delta_seconds } => {
                if let Err(error) = self.tick_projector_physics(delta_seconds) {
                    return vec![error_event(CoreErrorCode::Internal, error.to_string())];
                }
                self.display.mark_physics_tick(Instant::now());
                vec![CoreEvent::StateSnapshot {
                    state: self.snapshot(),
                }]
            }
            CoreCommand::ResetProjectorPhysics => {
                self.display.reset_physics(Instant::now());
                vec![CoreEvent::StateSnapshot {
                    state: self.snapshot(),
                }]
            }
            CoreCommand::StepTime { delta } => {
                self.transport
                    .step_time(delta, self.midi_length(), Instant::now());
                self.display.mark_physics_tick(Instant::now());
                self.audio_clock.set_time(self.transport.current_time());
                vec![CoreEvent::StateSnapshot {
                    state: self.snapshot(),
                }]
            }
            CoreCommand::SetPlaying { playing } => {
                let now = Instant::now();
                self.transport.set_playing(playing, now);
                self.display.mark_physics_tick(now);
                self.audio_clock.set_time(self.transport.current_time());
                self.audio_clock.set_playing(self.transport.playing());
                vec![CoreEvent::StateSnapshot {
                    state: self.snapshot(),
                }]
            }
            CoreCommand::TogglePlaying => {
                let now = Instant::now();
                self.transport.toggle_playing(now);
                self.display.mark_physics_tick(now);
                self.audio_clock.set_time(self.transport.current_time());
                self.audio_clock.set_playing(self.transport.playing());
                vec![CoreEvent::StateSnapshot {
                    state: self.snapshot(),
                }]
            }
            CoreCommand::SetSceneConfig { scene } => {
                self.display.set_scene_config(scene, Instant::now());
                if let Err(error) = self.refresh_note_colors() {
                    return vec![error_event(CoreErrorCode::Internal, error.to_string())];
                }
                self.snapshot_after_layout_validation()
            }
            CoreCommand::SetViewRange { seconds } => {
                self.display.set_view_range(seconds);
                vec![CoreEvent::StateSnapshot {
                    state: self.snapshot(),
                }]
            }
            CoreCommand::SetKeyRange {
                first_key,
                last_key,
            } => {
                self.display.set_key_range(first_key, last_key);
                self.snapshot_after_layout_validation()
            }
            CoreCommand::SetViewport { width, height } => {
                if let Err(error) = self
                    .display
                    .apply_viewport_overrides(Some(width), Some(height))
                {
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
            CoreCommand::Shutdown => {
                self.audio_session = None;
                vec![CoreEvent::ShutdownComplete]
            }
        }
    }

    fn start_audio_session(&mut self) {
        let Some(cache) = self.midi_cache.as_ref() else {
            return;
        };
        let Ok(audio_cache) = cache.audio_cache() else {
            return;
        };
        self.audio_session = Some(LiveAudioSession::spawn(
            audio_cache,
            Arc::clone(&self.audio_clock),
            Arc::clone(&self.audio_player),
        ));
    }

    fn restart_audio_session(&mut self) {
        self.audio_session = None;
        self.audio_clock = Arc::new(PlaybackClock::new());
        self.audio_clock.set_time(self.transport.current_time());
        self.audio_clock.set_playing(self.transport.playing());
        self.start_audio_session();
    }
}
