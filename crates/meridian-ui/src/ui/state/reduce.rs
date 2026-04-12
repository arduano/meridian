use std::sync::{Arc, Mutex};

use meridian_core::protocol::{AnalysisJobId, CoreEvent, MidiAnalysisJobStatus, ParsedMidiId};

use super::super::view_model::{AnalysisViewModel, UiViewModel};

pub(super) fn reduce_core_events(shared_state: &Arc<Mutex<UiViewModel>>, events: &[CoreEvent]) {
    let mut model = shared_state.lock().expect("shared UI state mutex poisoned");
    for event in events {
        reduce_core_event(&mut model, event);
    }
}

fn reduce_core_event(model: &mut UiViewModel, event: &CoreEvent) {
    match event {
        CoreEvent::StateSnapshot { state }
        | CoreEvent::MidiLoaded { state, .. }
        | CoreEvent::ProcessedMidiAttached { state, .. }
        | CoreEvent::DisplayCacheAttached { state, .. }
        | CoreEvent::AudioCacheAttached { state, .. }
        | CoreEvent::DisplaySessionAttached { state, .. }
        | CoreEvent::AudioSessionAttached { state, .. }
        | CoreEvent::FrameProjected { state, .. }
        | CoreEvent::FrameSaved { state, .. } => model.apply_snapshot(state),
        CoreEvent::ProcessedMidiBuilt {
            processed_midi_id,
            track_count,
            ..
        } => {
            model.analysis.processed_midi_id = Some(*processed_midi_id);
            model.analysis.track_count = Some(*track_count);
        }
        CoreEvent::MidiAnalysisJobStatus { status } => {
            apply_analysis_job_status(model, status);
        }
        CoreEvent::AudioStatus { status } => model.audio.status = status.clone(),
        CoreEvent::MidiProcess { event } => model.modify.latest_event = Some(event.clone()),
        CoreEvent::MidiProcessStatus { status } => model.modify.process_status = status.clone(),
        CoreEvent::VideoRenderStatus { status } => model.render_jobs.video = status.clone(),
        CoreEvent::AudioRenderStatus { status } => model.render_jobs.audio = status.clone(),
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

fn apply_analysis_job_status(model: &mut UiViewModel, status: &MidiAnalysisJobStatus) {
    match status {
        MidiAnalysisJobStatus::Running {
            job_id,
            parsed_midi_id,
            ..
        } => {
            model.analysis.pending_parsed_midi_id = Some(*parsed_midi_id);
            model.analysis.pending_job_id = Some(*job_id);
            model.analysis.processed_midi_id = None;
            model.analysis.data = None;
        }
        MidiAnalysisJobStatus::Finished {
            job_id,
            parsed_midi_id,
            result,
            ..
        } => {
            if analysis_status_matches_pending(&model.analysis, *job_id, *parsed_midi_id) {
                model.analysis.pending_parsed_midi_id = None;
                model.analysis.pending_job_id = None;
                model.analysis.processed_midi_id = None;
                model.analysis.data = Some(result.clone());
            }
        }
        MidiAnalysisJobStatus::Failed {
            job_id,
            parsed_midi_id,
            ..
        } => {
            if analysis_status_matches_pending(&model.analysis, *job_id, *parsed_midi_id) {
                model.analysis.pending_parsed_midi_id = None;
                model.analysis.pending_job_id = None;
                model.analysis.processed_midi_id = None;
                model.analysis.data = None;
            }
        }
    }
}

fn analysis_status_matches_pending(
    analysis: &AnalysisViewModel,
    job_id: AnalysisJobId,
    parsed_midi_id: ParsedMidiId,
) -> bool {
    analysis
        .pending_job_id
        .map(|pending| pending == job_id)
        .unwrap_or(true)
        || analysis.pending_parsed_midi_id == Some(parsed_midi_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use meridian_core::{
        audio::{AudioBackend, AudioConfig, AudioStatus},
        midi::analysis::{
            MidiAnalysisData, MidiAnalysisEventMetrics, MidiAnalysisFileMetrics,
            MidiAnalysisNoteMetrics, MidiAnalysisTempoMetrics,
        },
        midi::{MIDI_KEY_COUNT, MIDIAnalysisSummary},
        protocol::{AnalysisJobId, CoreEvent, MidiAnalysisJobStatus, ParsedMidiId, StateSnapshot},
        render::{DisplayTimeSpace, SceneLayout},
    };

    fn sample_state_snapshot() -> StateSnapshot {
        StateSnapshot {
            active_parsed_midi_id: None,
            active_processed_midi_id: None,
            active_display_cache_id: None,
            active_audio_cache_id: None,
            active_display_session_id: None,
            active_audio_session_id: None,
            active_video_render_job_id: None,
            active_audio_render_job_id: None,
            active_midi_process_job_id: None,
            midi_path: Some("test.mid".into()),
            midi_loaded: true,
            audio: AudioConfig::default(),
            audio_status: AudioStatus::default(),
            scene: SceneLayout::default().scene,
            current_time: 1.25,
            playing: true,
            midi_length: 2.5,
            total_notes: 32,
            view_range: 0.5,
            time_space: DisplayTimeSpace::Time,
            first_key: 12,
            last_key: 96,
            viewport_width: 1280,
            viewport_height: 720,
        }
    }

    fn sample_analysis_data() -> MidiAnalysisData {
        MidiAnalysisData {
            midi_length: 2.5,
            total_notes: 32,
            key_note_counts: vec![0; MIDI_KEY_COUNT],
            summary: MIDIAnalysisSummary {
                total_blocks: 0,
                keys_with_notes: 0,
                max_blocks_per_key: 0,
                max_notes_in_block: 0,
                densest_key: 0,
                densest_key_notes: 0,
            },
            buckets: Vec::new(),
            file: MidiAnalysisFileMetrics {
                source_bytes: 0,
                gzip_bytes: 0,
                gzip_ratio: 0.0,
                format: 0,
                declared_track_count: 0,
                actual_track_count: 0,
                ticks_per_quarter: None,
                total_event_count: 0,
            },
            events: MidiAnalysisEventMetrics::default(),
            notes: MidiAnalysisNoteMetrics::default(),
            tempo: MidiAnalysisTempoMetrics::default(),
        }
    }

    #[test]
    fn snapshot_events_update_shared_state() {
        let state = Arc::new(Mutex::new(UiViewModel::default()));
        reduce_core_events(
            &state,
            &[CoreEvent::StateSnapshot {
                state: sample_state_snapshot(),
            }],
        );

        let model = state.lock().expect("shared UI state mutex poisoned");
        assert_eq!(
            model.snapshot.as_ref().map(|snapshot| snapshot.midi_loaded),
            Some(true)
        );
        assert_eq!(model.transport.current_time, 1.25);
        assert_eq!(model.scene.viewport_width, 1280);
        assert!(matches!(model.audio.status.backend, AudioBackend::None));
        assert!(!model.audio.status.active);
    }

    #[test]
    fn analysis_job_status_tracks_pending_job_and_result() {
        let state = Arc::new(Mutex::new(UiViewModel::default()));
        let parsed_midi_id = ParsedMidiId(7);
        let job_id = AnalysisJobId(11);
        reduce_core_events(
            &state,
            &[CoreEvent::MidiAnalysisJobStatus {
                status: MidiAnalysisJobStatus::Running {
                    job_id,
                    parsed_midi_id,
                    kinds: Vec::new(),
                    progress: 0.25,
                    status: "working".into(),
                },
            }],
        );

        {
            let model = state.lock().expect("shared UI state mutex poisoned");
            assert_eq!(model.analysis.pending_job_id, Some(job_id));
            assert_eq!(model.analysis.pending_parsed_midi_id, Some(parsed_midi_id));
            assert!(model.analysis.data.is_none());
        }

        reduce_core_events(
            &state,
            &[CoreEvent::MidiAnalysisJobStatus {
                status: MidiAnalysisJobStatus::Finished {
                    job_id,
                    parsed_midi_id,
                    kinds: Vec::new(),
                    result: sample_analysis_data(),
                },
            }],
        );

        let model = state.lock().expect("shared UI state mutex poisoned");
        assert_eq!(model.analysis.pending_job_id, None);
        assert_eq!(model.analysis.pending_parsed_midi_id, None);
        assert!(model.analysis.data.is_some());
    }
}
