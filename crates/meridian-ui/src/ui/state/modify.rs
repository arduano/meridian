use meridian_core::protocol::{MidiProcessEvent, MidiProcessStatus};

use super::super::view::App;
use super::formatting::{file_name_or_full, format_number};

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

pub(super) fn apply_modify_process_to_app(
    app: &App,
    status: &MidiProcessStatus,
    latest_event: Option<&MidiProcessEvent>,
) {
    app.set_modify_job_active(matches!(
        status,
        MidiProcessStatus::Running { .. } | MidiProcessStatus::Cancelling { .. }
    ));
    app.set_modify_result_input_count_text("—".into());
    app.set_modify_result_track_count_text("—".into());
    app.set_modify_result_ppq_text("—".into());
    app.set_modify_result_event_count_text("—".into());

    match status {
        MidiProcessStatus::Running { output, .. } => {
            let active_progress = latest_active_progress(status, latest_event);
            app.set_modify_progress(active_progress.map(|(value, _)| value).unwrap_or(0.0));
            app.set_modify_status_text("Processing MIDI".into());
            app.set_modify_detail_text(
                active_progress
                    .map(|(_, label)| label.to_owned())
                    .unwrap_or_else(|| format!("Writing {}", output.display()))
                    .into(),
            );
            app.set_modify_result_output_text(file_name_or_full(output).into());
        }
        MidiProcessStatus::Cancelling { output, .. } => {
            let active_progress = latest_active_progress(status, latest_event);
            app.set_modify_progress(active_progress.map(|(value, _)| value).unwrap_or(0.0));
            app.set_modify_status_text("Cancelling".into());
            app.set_modify_detail_text(
                active_progress
                    .map(|(_, label)| label.to_owned())
                    .unwrap_or_else(|| format!("Stopping {}", output.display()))
                    .into(),
            );
            app.set_modify_result_output_text(file_name_or_full(output).into());
        }
        MidiProcessStatus::Idle => {
            app.set_modify_progress(0.0);
            match latest_event {
                Some(MidiProcessEvent::ProcessFinished {
                    output,
                    output_track_count,
                    output_ppq,
                    total_events,
                    ..
                }) => {
                    app.set_modify_status_text("Finished".into());
                    app.set_modify_detail_text(format!("Wrote {}", output.display()).into());
                    app.set_modify_result_output_text(file_name_or_full(output).into());
                    app.set_modify_result_input_count_text("1".into());
                    app.set_modify_result_track_count_text(
                        format_number(*output_track_count as u64).into(),
                    );
                    app.set_modify_result_ppq_text(output_ppq.to_string().into());
                    app.set_modify_result_event_count_text(
                        format_number(*total_events as u64).into(),
                    );
                }
                Some(MidiProcessEvent::ProcessFailed {
                    output, message, ..
                }) => {
                    app.set_modify_status_text("Failed".into());
                    app.set_modify_detail_text(message.clone().into());
                    app.set_modify_result_output_text(file_name_or_full(output).into());
                }
                Some(MidiProcessEvent::ProcessCancelled { output, .. }) => {
                    app.set_modify_status_text("Cancelled".into());
                    app.set_modify_detail_text("Processing stopped".into());
                    app.set_modify_result_output_text(file_name_or_full(output).into());
                }
                Some(MidiProcessEvent::ProcessStarted { output, .. }) => {
                    app.set_modify_status_text("Ready".into());
                    app.set_modify_detail_text(
                        format!("Configured to write {}", output.display()).into(),
                    );
                    app.set_modify_result_output_text(file_name_or_full(output).into());
                }
                Some(MidiProcessEvent::Progress { label, .. }) => {
                    app.set_modify_status_text("Ready".into());
                    app.set_modify_detail_text(label.clone().into());
                }
                None => {
                    app.set_modify_status_text("Ready".into());
                    app.set_modify_detail_text(
                        "Pick a pass, review the JSON config, and write a new MIDI file.".into(),
                    );
                    app.set_modify_result_output_text("output.mid".into());
                }
            }
        }
    }
}
