use meridian_core::protocol::{MidiProcessEvent, MidiProcessStatus};

use super::super::view::App;
use super::formatting::{file_name_or_full, format_number};

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
            app.set_modify_progress(0.0);
            app.set_modify_status_text("Processing MIDI".into());
            app.set_modify_detail_text(format!("Writing {}", output.display()).into());
            app.set_modify_result_output_text(file_name_or_full(output).into());
        }
        MidiProcessStatus::Cancelling { output, .. } => {
            app.set_modify_progress(0.0);
            app.set_modify_status_text("Cancelling".into());
            app.set_modify_detail_text(format!("Stopping {}", output.display()).into());
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
