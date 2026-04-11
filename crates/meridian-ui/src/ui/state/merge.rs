use std::sync::{Arc, Mutex};

use meridian_core::protocol::{MidiProcessEvent, MidiProcessStatus};
use slint::{ModelRc, VecModel};

use super::super::view::{App, MergeSourceRow};
use super::super::view_model::{MergeSourceInspection, UiViewModel};
use super::formatting::{file_name_or_full, format_duration_short, format_number};

fn latest_active_progress<'a>(
    status: &MidiProcessStatus,
    latest_event: Option<&'a MidiProcessEvent>,
) -> Option<(f32, &'a str)> {
    let job_id = match status {
        MidiProcessStatus::Running { job_id, .. }
        | MidiProcessStatus::Cancelling { job_id, .. } => *job_id,
        MidiProcessStatus::Idle => return None,
    };

    match latest_event {
        Some(MidiProcessEvent::Progress {
            job_id: progress_job_id,
            progress_percent,
            label,
        }) if *progress_job_id == job_id => Some((*progress_percent as f32 / 100.0, label)),
        _ => None,
    }
}

pub(super) fn apply_merge_sources_to_app(app: &App, shared_state: &Arc<Mutex<UiViewModel>>) {
    let sources = shared_state
        .lock()
        .expect("shared UI state mutex poisoned")
        .merge
        .sources
        .clone();

    let mut total_tracks = 0u64;
    let mut total_notes = 0u64;
    let mut total_tempos = 0u64;
    let rows = sources
        .into_iter()
        .map(|source| {
            let name = file_name_or_full(&source.path);
            match source.inspection {
                MergeSourceInspection::Loading => MergeSourceRow {
                    name: name.clone().into(),
                    path: source.path.display().to_string().into(),
                    status: "Inspecting".into(),
                    tracks_text: "…".into(),
                    notes_text: "…".into(),
                    tempo_text: "…".into(),
                    length_text: "…".into(),
                    bpm_text: "Reading MIDI metadata".into(),
                    ppq_text: "…".into(),
                    meta_text: "Track names / signatures pending".into(),
                    is_loading: true,
                    is_error: false,
                },
                MergeSourceInspection::Error(message) => MergeSourceRow {
                    name: name.clone().into(),
                    path: source.path.display().to_string().into(),
                    status: "Error".into(),
                    tracks_text: "—".into(),
                    notes_text: "—".into(),
                    tempo_text: "—".into(),
                    length_text: "—".into(),
                    bpm_text: message.clone().into(),
                    ppq_text: "—".into(),
                    meta_text: message.into(),
                    is_loading: false,
                    is_error: true,
                },
                MergeSourceInspection::Ready(inspection) => {
                    total_tracks += inspection.actual_track_count as u64;
                    total_notes += inspection.total_notes;
                    total_tempos += inspection.tempo_event_count;
                    MergeSourceRow {
                        name: name.clone().into(),
                        path: source.path.display().to_string().into(),
                        status: "Ready".into(),
                        tracks_text: format_number(inspection.actual_track_count as u64).into(),
                        notes_text: format_number(inspection.total_notes).into(),
                        tempo_text: format_number(inspection.tempo_event_count).into(),
                        length_text: format_duration_short(inspection.midi_length).into(),
                        bpm_text: if inspection.initial_bpm > 0.0 {
                            format!("{:.1} BPM", inspection.initial_bpm).into()
                        } else {
                            "No tempo events".into()
                        },
                        ppq_text: inspection
                            .ticks_per_quarter
                            .map(|ppq| ppq.to_string().into())
                            .unwrap_or_else(|| "SMPTE".into()),
                        meta_text: format!(
                            "{} track names / {} time sig / {} key sig",
                            format_number(inspection.track_name_event_count),
                            format_number(inspection.time_signature_event_count),
                            format_number(inspection.key_signature_event_count)
                        )
                        .into(),
                        is_loading: false,
                        is_error: false,
                    }
                }
            }
        })
        .collect::<Vec<_>>();

    app.set_merge_sources(ModelRc::from(std::rc::Rc::new(VecModel::from(rows))));
    app.set_merge_source_count_text(
        format_number(
            shared_state
                .lock()
                .expect("shared UI state mutex poisoned")
                .merge
                .sources
                .len() as u64,
        )
        .into(),
    );
    app.set_merge_track_total_text(format_number(total_tracks).into());
    app.set_merge_note_total_text(format_number(total_notes).into());
    app.set_merge_tempo_total_text(format_number(total_tempos).into());
}

pub(super) fn apply_merge_process_to_app(
    app: &App,
    status: &MidiProcessStatus,
    latest_event: Option<&MidiProcessEvent>,
) {
    app.set_merge_job_active(matches!(
        status,
        MidiProcessStatus::Running { .. } | MidiProcessStatus::Cancelling { .. }
    ));
    app.set_merge_result_input_count_text("—".into());
    app.set_merge_result_track_count_text("—".into());
    app.set_merge_result_ppq_text("—".into());
    app.set_merge_result_event_count_text("—".into());

    match status {
        MidiProcessStatus::Running { output, .. } => {
            let active_progress = latest_active_progress(status, latest_event);
            app.set_merge_progress(active_progress.map(|(value, _)| value).unwrap_or(0.0));
            app.set_merge_status_text("Processing MIDI".into());
            app.set_merge_detail_text(
                active_progress
                    .map(|(_, label)| label.to_owned())
                    .unwrap_or_else(|| format!("Writing {}", output.display()))
                    .into(),
            );
            app.set_merge_result_output_text(file_name_or_full(output).into());
        }
        MidiProcessStatus::Cancelling { output, .. } => {
            let active_progress = latest_active_progress(status, latest_event);
            app.set_merge_progress(active_progress.map(|(value, _)| value).unwrap_or(0.0));
            app.set_merge_status_text("Cancelling".into());
            app.set_merge_detail_text(
                active_progress
                    .map(|(_, label)| label.to_owned())
                    .unwrap_or_else(|| format!("Stopping {}", output.display()))
                    .into(),
            );
            app.set_merge_result_output_text(file_name_or_full(output).into());
        }
        MidiProcessStatus::Idle => {
            app.set_merge_progress(0.0);
            match latest_event {
                Some(MidiProcessEvent::ProcessFinished {
                    output,
                    output_track_count,
                    output_ppq,
                    total_events,
                    ..
                }) => {
                    app.set_merge_status_text("Finished".into());
                    app.set_merge_detail_text(format!("Wrote {}", output.display()).into());
                    app.set_merge_result_output_text(file_name_or_full(output).into());
                    app.set_merge_result_input_count_text("1".into());
                    app.set_merge_result_track_count_text(
                        format_number(*output_track_count as u64).into(),
                    );
                    app.set_merge_result_ppq_text(output_ppq.to_string().into());
                    app.set_merge_result_event_count_text(
                        format_number(*total_events as u64).into(),
                    );
                }
                Some(MidiProcessEvent::ProcessFailed {
                    output, message, ..
                }) => {
                    app.set_merge_status_text("Failed".into());
                    app.set_merge_detail_text(message.clone().into());
                    app.set_merge_result_output_text(file_name_or_full(output).into());
                }
                Some(MidiProcessEvent::ProcessCancelled { output, .. }) => {
                    app.set_merge_status_text("Cancelled".into());
                    app.set_merge_detail_text("Processing stopped".into());
                    app.set_merge_result_output_text(file_name_or_full(output).into());
                }
                Some(MidiProcessEvent::ProcessStarted { output, .. }) => {
                    app.set_merge_status_text("Ready".into());
                    app.set_merge_detail_text(
                        format!("Configured to write {}", output.display()).into(),
                    );
                    app.set_merge_result_output_text(file_name_or_full(output).into());
                }
                Some(MidiProcessEvent::Progress { label, .. }) => {
                    app.set_merge_status_text("Ready".into());
                    app.set_merge_detail_text(label.clone().into());
                }
                None => {
                    app.set_merge_status_text("Ready".into());
                    app.set_merge_detail_text(
                        "Add MIDI files, tune the merge recipe, and write a combined output."
                            .into(),
                    );
                    app.set_merge_result_output_text("merged-output.mid".into());
                }
            }
        }
    }
}
