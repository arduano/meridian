use super::*;

pub(in super::super) fn clear_export_state(export_state: &Arc<Mutex<RenderExportCoordinator>>) {
    *export_state
        .lock()
        .expect("render export coordinator mutex poisoned") = RenderExportCoordinator::default();
}

pub(in super::super) fn fail_export(
    app: &App,
    export_state: &Arc<Mutex<RenderExportCoordinator>>,
    message: String,
) {
    clear_export_state(export_state);
    set_export_status(app, "Failed", message, 0.0);
    app.window().request_redraw();
}

pub(in super::super) fn start_render_export_jobs(
    app: &App,
    bridge: &UiCoreBridge,
    shared_state: &Arc<Mutex<UiViewModel>>,
    export_state: &Arc<Mutex<RenderExportCoordinator>>,
) -> Result<(), String> {
    let snapshot = shared_state
        .lock()
        .expect("shared UI state mutex poisoned")
        .snapshot
        .clone()
        .ok_or_else(|| "load a MIDI before exporting".to_string())?;
    if snapshot.midi_path.is_none() {
        return Err("load a MIDI before exporting".into());
    }

    let mode = RenderExportMode::from_text(app.get_render_mode_text().as_str());
    let audio_format = AudioOnlyFormat::from_text(app.get_render_audio_format_text().as_str());
    let final_output = {
        let export = export_state
            .lock()
            .expect("render export coordinator mutex poisoned");
        if !export.active {
            return Err("export was cancelled".into());
        }
        export
            .final_output
            .clone()
            .ok_or_else(|| "missing final export path".to_string())?
    };

    let video_config = mode
        .wants_video()
        .then(|| build_video_render_config(app, &snapshot, final_output.clone(), mode))
        .transpose()?;
    let audio_config = (mode == RenderExportMode::AudioOnly)
        .then(|| build_audio_render_config(app, &snapshot, final_output.clone(), audio_format))
        .transpose()?;

    {
        let mut export = export_state
            .lock()
            .expect("render export coordinator mutex poisoned");
        if !export.active {
            return Err("export was cancelled".into());
        }
        export.mode = Some(mode);
        export.outcome = None;
    }

    if let Some(config) = video_config {
        let events = bridge
            .start_render_video(config, shared_state)
            .map_err(|error| error.to_string())?;
        if let Some(message) = events_error_message(&events) {
            return Err(message);
        }
        apply_events_to_app(app, shared_state, &events);
    }

    if let Some(config) = audio_config {
        let events = bridge
            .start_render_audio(config, shared_state)
            .map_err(|error| error.to_string())?;
        if let Some(message) = events_error_message(&events) {
            if app.get_video_render_status() != "Idle" {
                if let Ok(cancel_events) = bridge.cancel_render_video(shared_state) {
                    apply_events_to_app(app, shared_state, &cancel_events);
                }
            }
            return Err(message);
        }
        apply_events_to_app(app, shared_state, &events);
    }

    let status = match mode {
        RenderExportMode::VideoAudio => "Rendering video + audio",
        RenderExportMode::VideoOnly => "Rendering video",
        RenderExportMode::AudioOnly => "Rendering audio",
    };
    set_export_status(app, status, final_output.display().to_string(), 0.0);
    app.window().request_redraw();
    Ok(())
}
