use std::sync::{Arc, Mutex};

use meridian_core::protocol::{CoreEvent, StateSnapshot};
use slint::{ModelRc, VecModel};

use super::super::inspector::rows_for_scene;
use super::super::view::{App, MidiLoadState};
use super::super::view_model::UiViewModel;
use super::UiFrameUpdate;
use super::analysis::apply_analysis_to_app;
use super::audio::apply_audio_to_app;
use super::formatting::{file_name_or_full, format_view_range_label};
use super::merge::apply_merge_process_to_app;
use super::modify::apply_modify_process_to_app;
use super::video::{
    apply_video_render_status_to_app, apply_video_scene_to_app, renderer_summary, scene_summary,
};

pub(super) fn app_has_active_midi_load(app: &App) -> bool {
    app.get_render_load_state() == MidiLoadState::Loading
        || app.get_audio_load_state() == MidiLoadState::Loading
        || app.get_analysis_load_state() == MidiLoadState::Loading
}

pub(super) fn apply_events_to_app(
    app: &App,
    shared_state: &Arc<Mutex<UiViewModel>>,
    events: &[CoreEvent],
) {
    let snapshot = shared_state
        .lock()
        .expect("shared UI state mutex poisoned")
        .snapshot
        .clone();
    if let Some(state) = snapshot.as_ref() {
        apply_state_to_app(app, shared_state, state);
        for event in events {
            apply_event_overrides_to_app(app, event, state);
        }
    } else {
        for event in events {
            if let CoreEvent::Error { message, .. } = event {
                app.set_status_text(message.clone().into());
            }
        }
    }
}

pub(super) fn apply_frame_update_to_app(
    app: &App,
    shared_state: &Arc<Mutex<UiViewModel>>,
    update: &UiFrameUpdate,
) {
    apply_state_to_app(app, shared_state, &update.state);
    app.set_visible_note_count_text(update.visible_notes.to_string().into());
    app.set_active_keys_text(update.active_keys.to_string().into());
    app.set_fps_text(update.fps_text.clone().into());
    app.set_status_text(status_text(&update.state).into());
}

pub(super) fn apply_event_overrides_to_app(app: &App, event: &CoreEvent, state: &StateSnapshot) {
    match event {
        CoreEvent::StateSnapshot { .. }
        | CoreEvent::MidiLoaded { .. }
        | CoreEvent::ProcessedMidiAttached { .. }
        | CoreEvent::DisplayCacheAttached { .. }
        | CoreEvent::AudioCacheAttached { .. } => {
            app.set_status_text(status_text(state).into());
            app.set_visible_note_count_text("0".into());
            app.set_active_keys_text("0".into());
        }
        CoreEvent::ProcessedMidiBuilt {
            midi_length,
            total_notes,
            track_count,
            ..
        } => {
            app.set_analysis_note_count_text(total_notes.to_string().into());
            app.set_analysis_track_count_text(track_count.to_string().into());
            app.set_analysis_midi_length_text(format!("{:.3} s", midi_length).into());
        }
        CoreEvent::MidiAnalysisJobStatus { status } => {
            if let meridian_core::protocol::MidiAnalysisJobStatus::Finished { result, .. } = status
            {
                apply_analysis_to_app(app, result);
            }
        }
        CoreEvent::FrameProjected { stats, .. } => {
            app.set_visible_note_count_text(stats.visible_notes.to_string().into());
            app.set_active_keys_text(stats.active_keys.to_string().into());
            app.set_status_text(status_text(state).into());
        }
        CoreEvent::FrameSaved { output, .. } => {
            app.set_status_text(format!("Saved frame to {}", output.display()).into());
        }
        CoreEvent::MidiFileProcessed { output, .. } => {
            app.set_modify_status_text("Finished".into());
            app.set_modify_detail_text(format!("Wrote {}", output.display()).into());
            app.set_modify_result_output_text(file_name_or_full(output).into());
        }
        CoreEvent::MidiFilesMerged { output, .. } => {
            app.set_merge_status_text("Finished".into());
            app.set_merge_detail_text(format!("Wrote {}", output.display()).into());
            app.set_merge_result_output_text(file_name_or_full(output).into());
        }
        CoreEvent::VideoRender { .. }
        | CoreEvent::VideoRenderStatus { .. }
        | CoreEvent::AudioRender { .. }
        | CoreEvent::AudioRenderStatus { .. }
        | CoreEvent::AudioStatus { .. }
        | CoreEvent::MidiLoadProgress { .. }
        | CoreEvent::ParsedMidiLoaded { .. }
        | CoreEvent::MidiFilesInspected { .. }
        | CoreEvent::DisplayCacheBuilt { .. }
        | CoreEvent::AudioCacheBuilt { .. }
        | CoreEvent::DisplaySessionCreated { .. }
        | CoreEvent::AudioSessionCreated { .. }
        | CoreEvent::MidiAnalysisJob { .. }
        | CoreEvent::MidiProcess { .. }
        | CoreEvent::MidiProcessStatus { .. } => {}
        CoreEvent::DisplaySessionAttached { .. } | CoreEvent::AudioSessionAttached { .. } => {
            app.set_status_text(status_text(state).into());
        }
        CoreEvent::Error { message, .. } => {
            app.set_status_text(message.clone().into());
        }
        CoreEvent::ShutdownComplete => {}
    }
}

pub(super) fn apply_state_to_app(
    app: &App,
    shared_state: &Arc<Mutex<UiViewModel>>,
    state: &StateSnapshot,
) {
    let (audio_render_status, video_render_status, modify_process_status, modify_latest_event) = {
        let mut model = shared_state.lock().expect("shared UI state mutex poisoned");
        model.apply_snapshot(state);
        (
            model.render_jobs.audio.clone(),
            model.render_jobs.video.clone(),
            model.modify.process_status.clone(),
            model.modify.latest_event.clone(),
        )
    };
    app.set_midi_path_text(
        state
            .midi_path
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "No MIDI loaded".into())
            .into(),
    );
    app.set_time_text(format!("{:.3} s", state.current_time).into());
    app.set_length_text(format!("{:.3} s", state.midi_length).into());
    app.set_note_count_text(state.total_notes.to_string().into());
    app.set_view_range_text(format_view_range_label(state.view_range).into());
    app.set_current_time_seconds(state.current_time as f32);
    app.set_midi_length_seconds(state.midi_length.max(0.001) as f32);
    app.set_scene_summary_text(scene_summary(&state.scene).into());
    app.set_current_renderer_text(renderer_summary(&state.scene).into());
    app.set_time_space_text(match state.time_space {
        meridian_core::render::DisplayTimeSpace::Time => "time".into(),
        meridian_core::render::DisplayTimeSpace::Tick => "tick".into(),
    });
    app.set_viewport_text(format!("{} x {}", state.viewport_width, state.viewport_height).into());
    app.set_inspector_items(ModelRc::from(std::rc::Rc::new(VecModel::from(
        rows_for_scene(&state.scene),
    ))));
    apply_video_scene_to_app(app, state);
    app.set_play_label(if state.playing {
        "Pause".into()
    } else {
        "Play".into()
    });
    apply_audio_to_app(app, state, &audio_render_status);
    apply_video_render_status_to_app(app, &video_render_status);
    apply_modify_process_to_app(app, &modify_process_status, modify_latest_event.as_ref());
    apply_merge_process_to_app(app, &modify_process_status, modify_latest_event.as_ref());
}

pub(super) fn status_text(state: &StateSnapshot) -> String {
    if let Some(path) = &state.midi_path {
        format!(
            "Core session active. UI frontend attached. Rendering {} through the shared event bus.",
            path.display()
        )
    } else {
        "Launch with `meridian-ui --midi <file.mid>` to render a MIDI".into()
    }
}
