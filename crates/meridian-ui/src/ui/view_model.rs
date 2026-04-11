use std::{
    path::PathBuf,
    sync::{Arc, atomic::AtomicBool},
};

use meridian_core::{
    audio::AudioStatus,
    midi::MidiFileInspection,
    protocol::{
        AnalysisJobId, AudioRenderStatus, CoreEvent, MidiAnalysisData, MidiAnalysisJobStatus,
        MidiProcessEvent, MidiProcessStatus, ParsedMidiId, ProcessedMidiId, StateSnapshot,
        VideoRenderStatus,
    },
    render::{DisplayTimeSpace, SceneConfig},
};

#[derive(Debug, Clone, Default)]
pub struct TransportViewModel {
    pub current_time: f64,
    pub midi_length: f64,
    pub playing: bool,
    pub view_range: f64,
    pub time_space: DisplayTimeSpace,
}

#[derive(Debug, Clone, Default)]
pub struct SceneViewModel {
    pub midi_path: Option<PathBuf>,
    pub midi_loaded: bool,
    pub scene: Option<SceneConfig>,
    pub total_notes: u64,
    pub first_key: u8,
    pub last_key: u8,
    pub viewport_width: u32,
    pub viewport_height: u32,
}

#[derive(Debug, Clone, Default)]
pub struct AudioViewModel {
    pub status: AudioStatus,
}

#[derive(Debug, Clone)]
pub struct RenderJobsViewModel {
    pub video: VideoRenderStatus,
    pub audio: AudioRenderStatus,
}

impl Default for RenderJobsViewModel {
    fn default() -> Self {
        Self {
            video: VideoRenderStatus::Idle,
            audio: AudioRenderStatus::Idle,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct UiViewModel {
    pub transport: TransportViewModel,
    pub scene: SceneViewModel,
    pub audio: AudioViewModel,
    pub render_jobs: RenderJobsViewModel,
    pub analysis: AnalysisViewModel,
    pub modify: ModifyViewModel,
    pub merge: MergeViewModel,
    pub snapshot: Option<StateSnapshot>,
}

#[derive(Debug, Clone, Default)]
pub struct AnalysisViewModel {
    pub processed_midi_id: Option<ProcessedMidiId>,
    pub data: Option<MidiAnalysisData>,
    pub track_count: Option<usize>,
    pub pending_parsed_midi_id: Option<ParsedMidiId>,
    pub pending_job_id: Option<AnalysisJobId>,
}

#[derive(Debug, Clone)]
pub struct ModifyViewModel {
    pub process_status: MidiProcessStatus,
    pub latest_event: Option<MidiProcessEvent>,
}

impl Default for ModifyViewModel {
    fn default() -> Self {
        Self {
            process_status: MidiProcessStatus::Idle,
            latest_event: None,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct MergeViewModel {
    pub sources: Vec<MergeSourceViewModel>,
    pub cancel: Option<Arc<AtomicBool>>,
}

#[derive(Debug, Clone)]
pub struct MergeSourceViewModel {
    pub path: PathBuf,
    pub inspection: MergeSourceInspection,
}

#[derive(Debug, Clone)]
pub enum MergeSourceInspection {
    Loading,
    Ready(MidiFileInspection),
    Error(String),
}

impl MergeSourceViewModel {
    pub fn new_loading(path: PathBuf) -> Self {
        Self {
            path,
            inspection: MergeSourceInspection::Loading,
        }
    }
}

impl UiViewModel {
    pub fn clear_analysis(&mut self) {
        self.analysis = AnalysisViewModel::default();
    }

    pub fn reduce_events(&mut self, events: &[CoreEvent]) {
        for event in events {
            self.reduce_event(event);
        }
    }

    pub fn apply_snapshot(&mut self, state: &StateSnapshot) {
        self.snapshot = Some(state.clone());
        self.transport.current_time = state.current_time;
        self.transport.midi_length = state.midi_length;
        self.transport.playing = state.playing;
        self.transport.view_range = state.view_range;
        self.transport.time_space = state.time_space;

        self.scene.midi_path = state.midi_path.clone();
        self.scene.midi_loaded = state.midi_loaded;
        self.scene.scene = Some(state.scene.clone());
        self.scene.total_notes = state.total_notes;
        self.scene.first_key = state.first_key;
        self.scene.last_key = state.last_key;
        self.scene.viewport_width = state.viewport_width;
        self.scene.viewport_height = state.viewport_height;

        self.audio.status = state.audio_status.clone();
    }

    fn reduce_event(&mut self, event: &CoreEvent) {
        match event {
            CoreEvent::StateSnapshot { state }
            | CoreEvent::MidiLoaded { state, .. }
            | CoreEvent::ProcessedMidiAttached { state, .. }
            | CoreEvent::DisplayCacheAttached { state, .. }
            | CoreEvent::AudioCacheAttached { state, .. }
            | CoreEvent::DisplaySessionAttached { state, .. }
            | CoreEvent::AudioSessionAttached { state, .. }
            | CoreEvent::FrameProjected { state, .. }
            | CoreEvent::FrameSaved { state, .. } => self.apply_snapshot(state),
            CoreEvent::ProcessedMidiBuilt {
                processed_midi_id,
                track_count,
                ..
            } => {
                self.analysis.processed_midi_id = Some(*processed_midi_id);
                self.analysis.track_count = Some(*track_count);
            }
            CoreEvent::MidiAnalysisJobStatus { status } => match status {
                MidiAnalysisJobStatus::Running {
                    job_id,
                    parsed_midi_id,
                    ..
                } => {
                    self.analysis.pending_parsed_midi_id = Some(*parsed_midi_id);
                    self.analysis.pending_job_id = Some(*job_id);
                    self.analysis.processed_midi_id = None;
                    self.analysis.data = None;
                }
                MidiAnalysisJobStatus::Finished {
                    job_id,
                    parsed_midi_id,
                    result,
                    ..
                } => {
                    if self
                        .analysis
                        .pending_job_id
                        .map(|pending| pending == *job_id)
                        .unwrap_or(true)
                        || self.analysis.pending_parsed_midi_id == Some(*parsed_midi_id)
                    {
                        self.analysis.pending_parsed_midi_id = None;
                        self.analysis.pending_job_id = None;
                        self.analysis.processed_midi_id = None;
                        self.analysis.data = Some(result.clone());
                    }
                }
                MidiAnalysisJobStatus::Failed {
                    job_id,
                    parsed_midi_id,
                    ..
                } => {
                    if self
                        .analysis
                        .pending_job_id
                        .map(|pending| pending == *job_id)
                        .unwrap_or(true)
                        || self.analysis.pending_parsed_midi_id == Some(*parsed_midi_id)
                    {
                        self.analysis.pending_parsed_midi_id = None;
                        self.analysis.pending_job_id = None;
                        self.analysis.processed_midi_id = None;
                        self.analysis.data = None;
                    }
                }
            },
            CoreEvent::AudioStatus { status } => self.audio.status = status.clone(),
            CoreEvent::MidiProcess { event } => self.modify.latest_event = Some(event.clone()),
            CoreEvent::MidiProcessStatus { status } => self.modify.process_status = status.clone(),
            CoreEvent::VideoRenderStatus { status } => self.render_jobs.video = status.clone(),
            CoreEvent::AudioRenderStatus { status } => self.render_jobs.audio = status.clone(),
            CoreEvent::VideoRender { .. }
            | CoreEvent::AudioRender { .. }
            | CoreEvent::MidiLoadProgress { .. }
            | CoreEvent::ParsedMidiLoaded { .. }
            | CoreEvent::MidiFilesInspected { .. }
            | CoreEvent::DisplayCacheBuilt { .. }
            | CoreEvent::AudioCacheBuilt { .. }
            | CoreEvent::DisplaySessionCreated { .. }
            | CoreEvent::AudioSessionCreated { .. }
            | CoreEvent::MidiAnalysisJob { .. }
            | CoreEvent::MidiFileProcessed { .. }
            | CoreEvent::MidiFilesMerged { .. }
            | CoreEvent::Error { .. }
            | CoreEvent::ShutdownComplete => {}
        }
    }
}
