use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64},
    },
    thread::JoinHandle,
    time::Instant,
};

use flume::{Receiver, Sender};

use crate::{
    audio::{AudioConfig, LiveAudioSession, MeridianAudioPlayer, PlaybackClock},
    display::LiveDisplaySession,
    midi::audio_cache::InRamAudioCache,
    midi::{MidiCacheStack, ProcessedMidi},
    protocol::{
        AnalysisJobId, AudioCacheId, AudioRenderJobId, AudioRenderStatus, AudioSessionId,
        CoreCommand, CoreErrorCode, CoreEvent, DisplayCacheId, DisplaySessionId,
        MidiAnalysisJobStatus, MidiProcessJobId, MidiProcessStatus, ParsedMidiId, ProcessedMidiId,
        VideoRenderJobId, VideoRenderStatus,
    },
    transport::TransportState,
};

use super::{
    CoreHandle, CoreResponse, RequestMessage,
    resource_types::{
        AudioCacheRegistry, AudioSessionRegistry, DisplayCacheRegistry, DisplaySessionRegistry,
        ParsedMidiRegistry, ProcessedMidiRegistry,
    },
    support::error_event,
};

pub(super) struct RenderJobState {
    pub(super) cancel: Arc<AtomicBool>,
    pub(super) worker: Option<JoinHandle<()>>,
    pub(super) status: VideoRenderStatus,
}

pub(super) struct AudioRenderJobState {
    pub(super) cancel: Arc<AtomicBool>,
    pub(super) worker: Option<JoinHandle<()>>,
    pub(super) status: AudioRenderStatus,
}

pub(super) struct MidiProcessJobState {
    pub(super) cancel: Arc<AtomicBool>,
    pub(super) status: MidiProcessStatus,
}

pub(super) struct CoreState {
    pub(super) core_handle: CoreHandle,
    pub(super) subscribers: Arc<Mutex<Vec<Sender<CoreEvent>>>>,
    pub(super) midi_cache: Option<MidiCacheStack>,
    pub(super) processed_midi: Option<Arc<ProcessedMidi>>,
    pub(super) parsed_midis: ParsedMidiRegistry,
    pub(super) processed_midis: ProcessedMidiRegistry,
    pub(super) display_caches: DisplayCacheRegistry,
    pub(super) audio_caches: AudioCacheRegistry,
    pub(super) display_sessions: DisplaySessionRegistry,
    pub(super) audio_sessions: AudioSessionRegistry,
    pub(super) next_resource_id: u64,
    pub(super) active_parsed_midi_id: Option<ParsedMidiId>,
    pub(super) active_processed_midi_id: Option<ProcessedMidiId>,
    pub(super) active_display_cache_id: Option<DisplayCacheId>,
    pub(super) active_audio_cache_id: Option<AudioCacheId>,
    pub(super) active_display_session_id: Option<DisplaySessionId>,
    pub(super) active_audio_session_id: Option<AudioSessionId>,
    pub(super) active_video_render_job_id: Option<VideoRenderJobId>,
    pub(super) active_audio_render_job_id: Option<AudioRenderJobId>,
    pub(super) active_midi_process_job_id: Option<MidiProcessJobId>,
    pub(super) current_audio_cache: Option<Arc<InRamAudioCache>>,
    pub(super) analysis_jobs: HashMap<AnalysisJobId, MidiAnalysisJobStatus>,
    pub(super) display: LiveDisplaySession,
    pub(super) audio_config: AudioConfig,
    pub(super) audio_player: Arc<MeridianAudioPlayer>,
    pub(super) audio_clock: Arc<PlaybackClock>,
    pub(super) audio_session: Option<LiveAudioSession>,
    pub(super) midi_path: Option<PathBuf>,
    pub(super) transport: TransportState,
    pub(super) render_job: Option<RenderJobState>,
    pub(super) audio_render_job: Option<AudioRenderJobState>,
    pub(super) midi_process_job: Option<MidiProcessJobState>,
}

impl CoreState {
    pub(super) fn new(
        sender: Sender<RequestMessage>,
        subscribers: Arc<Mutex<Vec<Sender<CoreEvent>>>>,
        midi_load_generation: Arc<AtomicU64>,
    ) -> Self {
        Self {
            core_handle: CoreHandle {
                sender,
                subscribers: Arc::clone(&subscribers),
                midi_load_generation,
            },
            subscribers,
            midi_cache: None,
            processed_midi: None,
            parsed_midis: HashMap::new(),
            processed_midis: HashMap::new(),
            display_caches: HashMap::new(),
            audio_caches: HashMap::new(),
            display_sessions: HashMap::new(),
            audio_sessions: HashMap::new(),
            next_resource_id: 1,
            active_parsed_midi_id: None,
            active_processed_midi_id: None,
            active_display_cache_id: None,
            active_audio_cache_id: None,
            active_display_session_id: None,
            active_audio_session_id: None,
            active_video_render_job_id: None,
            active_audio_render_job_id: None,
            active_midi_process_job_id: None,
            current_audio_cache: None,
            analysis_jobs: HashMap::new(),
            display: LiveDisplaySession::new(),
            audio_config: AudioConfig::default(),
            audio_player: MeridianAudioPlayer::new(&AudioConfig::default()),
            audio_clock: Arc::new(PlaybackClock::new()),
            audio_session: None,
            midi_path: None,
            transport: TransportState::new(),
            render_job: None,
            audio_render_job: None,
            midi_process_job: None,
        }
    }

    pub(super) fn run(&mut self, receiver: Receiver<RequestMessage>) {
        let mut shutdown_reply: Option<Sender<CoreResponse>> = None;

        for request in receiver {
            if shutdown_reply.is_some() {
                match request {
                    RequestMessage::VideoRenderUpdate { event } => {
                        self.handle_video_render_update(event);
                    }
                    RequestMessage::AudioRenderUpdate { event } => {
                        self.handle_audio_render_update(event);
                    }
                    RequestMessage::MidiProcessUpdate { event } => {
                        self.handle_midi_process_update(event);
                    }
                    RequestMessage::AnalysisJobUpdate { event } => {
                        self.handle_midi_analysis_job_update(event);
                    }
                    RequestMessage::Command { command, reply } => {
                        if matches!(command, CoreCommand::Shutdown) {
                            let _ = reply.send(vec![error_event(
                                CoreErrorCode::InvalidCommand,
                                "shutdown in progress",
                            )]);
                        } else {
                            let response = self.handle(command);
                            let _ = reply.send(response);
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

                if !self.has_active_render_workers() {
                    let response = self.finish_shutdown();
                    if let Some(reply) = shutdown_reply.take() {
                        let _ = reply.send(response);
                    }
                    break;
                }
                continue;
            }

            match request {
                RequestMessage::Command { command, reply } => {
                    if matches!(command, CoreCommand::Shutdown) {
                        self.begin_shutdown();
                        if self.has_active_render_workers() {
                            shutdown_reply = Some(reply);
                        } else {
                            let _ = reply.send(self.finish_shutdown());
                            break;
                        }
                    } else {
                        let response = self.handle(command);
                        let _ = reply.send(response);
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
                    if shutdown_reply.is_some() && !self.has_active_render_workers() {
                        let response = self.finish_shutdown();
                        if let Some(reply) = shutdown_reply.take() {
                            let _ = reply.send(response);
                        }
                        break;
                    }
                }
                RequestMessage::AudioRenderUpdate { event } => {
                    self.handle_audio_render_update(event);
                    if shutdown_reply.is_some() && !self.has_active_render_workers() {
                        let response = self.finish_shutdown();
                        if let Some(reply) = shutdown_reply.take() {
                            let _ = reply.send(response);
                        }
                        break;
                    }
                }
                RequestMessage::MidiProcessUpdate { event } => {
                    self.handle_midi_process_update(event);
                }
                RequestMessage::AnalysisJobUpdate { event } => {
                    self.handle_midi_analysis_job_update(event);
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
            CoreCommand::LoadParsedMidi { path } => self.load_parsed_midi_resource(path),
            CoreCommand::InspectMidiFiles { paths } => vec![CoreEvent::MidiFilesInspected {
                inspections: crate::midi::inspect::inspect_midi_files(&paths),
            }],
            CoreCommand::BuildProcessedMidi {
                parsed_midi_id,
                config,
            } => self.build_processed_midi_resource(parsed_midi_id, config),
            CoreCommand::ProcessMidiFile {
                input,
                output,
                config,
            } => match crate::midi::file_processing::process_midi_file_to_file(
                &input, &output, &config,
            ) {
                Ok(summary) => vec![CoreEvent::MidiFileProcessed {
                    input: summary.input,
                    output: summary.output,
                    output_track_count: summary.output_track_count,
                    output_ppq: summary.output_ppq,
                    total_events: summary.total_events,
                }],
                Err(error) => vec![error_event(super::error_code(&error), error.to_string())],
            },
            CoreCommand::MergeMidiFiles {
                inputs,
                output,
                config,
            } => match crate::midi::file_merge::merge_midi_files_to_file(&inputs, &output, &config)
            {
                Ok(summary) => vec![CoreEvent::MidiFilesMerged {
                    output: summary.output,
                    input_count: summary.input_count,
                    output_track_count: summary.output_track_count,
                    output_ppq: summary.output_ppq,
                    total_events: summary.total_events,
                }],
                Err(error) => vec![error_event(super::error_code(&error), error.to_string())],
            },
            CoreCommand::StartProcessMidiFile {
                input,
                output,
                config,
            } => self.start_process_midi_file(input, output, config),
            CoreCommand::CancelMidiFileProcess => self.cancel_midi_file_process(),
            CoreCommand::GetMidiFileProcessStatus => vec![CoreEvent::MidiProcessStatus {
                status: self.midi_process_status(),
            }],
            CoreCommand::StartMidiAnalysisJob {
                parsed_midi_id,
                kinds,
                bucket_count,
            } => self.start_midi_analysis_job(parsed_midi_id, kinds, bucket_count),
            CoreCommand::BuildDisplayCache { parsed_midi_id } => {
                self.build_display_cache_resource(parsed_midi_id)
            }
            CoreCommand::BuildAudioCache { parsed_midi_id } => {
                self.build_audio_cache_resource(parsed_midi_id)
            }
            CoreCommand::CreateDisplaySession { display_cache_id } => {
                self.create_display_session_resource(display_cache_id)
            }
            CoreCommand::CreateAudioSession { audio_cache_id } => {
                self.create_audio_session_resource(audio_cache_id)
            }
            CoreCommand::AttachDisplayCache { display_cache_id } => {
                self.attach_display_cache_resource(display_cache_id)
            }
            CoreCommand::AttachProcessedMidi { processed_midi_id } => {
                self.attach_processed_midi_resource(processed_midi_id)
            }
            CoreCommand::AttachAudioCache { audio_cache_id } => {
                self.attach_audio_cache_resource(audio_cache_id)
            }
            CoreCommand::AttachDisplaySession { display_session_id } => {
                self.attach_display_session_resource(display_session_id)
            }
            CoreCommand::AttachAudioSession { audio_session_id } => {
                self.attach_audio_session_resource(audio_session_id)
            }
            CoreCommand::LoadDisplayMidi { path } => self.load_display_midi(path),
            CoreCommand::LoadAudioMidi { path } => self.load_audio_midi(path),
            CoreCommand::UnloadDisplayContext => {
                let cancel_events = self.cancel_active_render_jobs(true, false);
                self.broadcast_internal(cancel_events);
                self.active_video_render_job_id = self.active_video_render_job_id_from_state();
                self.unload_display_context()
            }
            CoreCommand::UnloadAudioContext => {
                let cancel_events = self.cancel_active_render_jobs(false, true);
                self.broadcast_internal(cancel_events);
                let events = self.unload_audio_context();
                self.active_audio_render_job_id = self.active_audio_render_job_id_from_state();
                events
            }
            CoreCommand::UnloadRenderContext => {
                let cancel_events = self.cancel_active_render_jobs(true, true);
                self.broadcast_internal(cancel_events);
                let events = self.unload_render_context();
                self.active_video_render_job_id = self.active_video_render_job_id_from_state();
                self.active_audio_render_job_id = self.active_audio_render_job_id_from_state();
                events
            }
            CoreCommand::DropInactiveMidiResources => self.drop_inactive_midi_resources(),
            CoreCommand::LoadMidi { path } => self.load_midi_legacy(path),
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
            CoreCommand::SetViewRange {
                seconds,
                time_space,
            } => {
                self.display.set_view_range(seconds, time_space);
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
                export,
            } => {
                let format = format.unwrap_or_else(|| {
                    crate::protocol::ImageOutputFormat::infer_from_path(&output)
                });
                self.save_frame_headless(output, format, viewport_width, viewport_height, export)
            }
            CoreCommand::StartRenderVideo { config } => self.start_render_video(config),
            CoreCommand::CancelRenderVideo => self.cancel_render_video(),
            CoreCommand::GetRenderVideoStatus => vec![CoreEvent::VideoRenderStatus {
                status: self.video_render_status(),
            }],
            CoreCommand::Shutdown => self.finish_shutdown(),
        }
    }

    pub(super) fn start_audio_session(&mut self) {
        let Some(audio_cache) = self.current_audio_cache.clone() else {
            return;
        };
        self.audio_session = Some(LiveAudioSession::spawn(
            audio_cache,
            Arc::clone(&self.audio_clock),
            Arc::clone(&self.audio_player),
        ));
    }

    pub(super) fn restart_audio_session(&mut self) {
        self.audio_session = None;
        self.audio_clock = Arc::new(PlaybackClock::new());
        self.audio_clock.set_time(self.transport.current_time());
        self.audio_clock.set_playing(self.transport.playing());
        self.start_audio_session();
    }

    fn broadcast_internal(&self, events: Vec<CoreEvent>) {
        for event in events {
            self.broadcast(event);
        }
    }

    fn begin_shutdown(&mut self) {
        let cancel_events = self.cancel_active_render_jobs(true, true);
        self.broadcast_internal(cancel_events);
        self.active_video_render_job_id = self.active_video_render_job_id_from_state();
        self.active_audio_render_job_id = self.active_audio_render_job_id_from_state();
    }

    fn finish_shutdown(&mut self) -> Vec<CoreEvent> {
        self.audio_session = None;
        self.midi_process_job = None;
        self.active_midi_process_job_id = None;
        vec![CoreEvent::ShutdownComplete]
    }

    fn has_active_render_workers(&self) -> bool {
        self.render_job.is_some() || self.audio_render_job.is_some()
    }
}
