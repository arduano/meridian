use std::sync::{Arc, Mutex};

use slint::{ModelRc, VecModel};

use super::super::view::{App, MergeSourceRow};
use super::super::view_model::{MergeSourceInspection, UiViewModel};
use super::formatting::{file_name_or_full, format_duration_short, format_number};

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
