use super::*;

pub(super) fn update_merge_control(app: &App, key: &str, value: &str) {
    match key {
        "layout" => app.set_merge_layout_text(value.into()),
        "metadata_mode" => app.set_merge_metadata_mode_text(value.into()),
        "ppq_override" => app.set_merge_ppq_override_text(value.into()),
        _ => {}
    }
}

pub(super) fn build_merge_config(app: &App) -> MidiFilesMergeConfig {
    let mode = match app.get_merge_layout_text().as_str() {
        "merge_tracks" => MidiFilesMergeMode::MergeTracks,
        _ => MidiFilesMergeMode::AppendTracks,
    };

    let normalize_metadata_track = app.get_merge_metadata_mode_text().as_str() != "keep";
    let ppq_override = parse_optional_value(
        app.get_merge_ppq_override_text().as_str(),
        "merge PPQ override",
    )
    .unwrap_or(None);

    MidiFilesMergeConfig {
        mode,
        normalize_metadata_track,
        ppq_override,
    }
}

pub(super) fn file_name_or_path(path: &Path) -> String {
    path.file_name()
        .and_then(OsStr::to_str)
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| path.display().to_string())
}

pub(super) fn default_merge_output_path(inputs: &[PathBuf]) -> PathBuf {
    let Some(first) = inputs.first() else {
        return PathBuf::from("merged-output.mid");
    };
    let stem = first
        .file_stem()
        .and_then(OsStr::to_str)
        .filter(|name| !name.is_empty())
        .unwrap_or("merged-output");
    let suffix = if inputs.len() > 1 {
        format!("-plus-{}", inputs.len() - 1)
    } else {
        String::new()
    };
    let file_name = format!("{stem}{suffix}-merged.mid");
    first
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .map(|parent| parent.join(file_name.clone()))
        .unwrap_or_else(|| PathBuf::from(file_name))
}

pub(super) fn normalize_merge_output_path(path: &Path) -> PathBuf {
    normalize_midi_output_path(path)
}

fn normalize_midi_output_path(path: &Path) -> PathBuf {
    let mut normalized = path.to_path_buf();
    let has_midi_extension = normalized
        .extension()
        .and_then(OsStr::to_str)
        .is_some_and(|ext| ext.eq_ignore_ascii_case("mid") || ext.eq_ignore_ascii_case("midi"));
    if !has_midi_extension {
        normalized.set_extension("mid");
    }
    normalized
}

pub(super) fn current_merge_output_path(app: &App) -> Result<PathBuf, String> {
    let raw = app.get_merge_output_path_text();
    if raw.is_empty() {
        return Err("choose an output path".into());
    }
    Ok(normalize_merge_output_path(Path::new(raw.as_str())))
}

fn resolve_midi_job_path(path: &Path) -> PathBuf {
    let normalized = normalize_midi_output_path(path);
    if normalized.is_absolute() {
        normalized
    } else {
        std::env::current_dir()
            .map(|cwd| cwd.join(&normalized))
            .unwrap_or(normalized)
    }
}

fn midi_paths_conflict(input: &Path, output: &Path) -> bool {
    let resolved_input = resolve_midi_job_path(input);
    let resolved_output = resolve_midi_job_path(output);
    if resolved_input == resolved_output {
        return true;
    }

    match (
        std::fs::canonicalize(&resolved_input),
        std::fs::canonicalize(&resolved_output),
    ) {
        (Ok(canonical_input), Ok(canonical_output)) => canonical_input == canonical_output,
        _ => false,
    }
}

pub(super) fn validate_modify_job_output_path(
    selected: &Path,
    output: &Path,
) -> Result<(), String> {
    if midi_paths_conflict(selected, output) {
        Err(format!(
            "output path must differ from the selected input MIDI ({})",
            file_name_or_path(selected)
        ))
    } else {
        Ok(())
    }
}

pub(super) fn validate_merge_job_output_path(
    inputs: &[PathBuf],
    output: &Path,
) -> Result<(), String> {
    if let Some(conflict) = inputs
        .iter()
        .find(|input| midi_paths_conflict(input, output))
    {
        Err(format!(
            "output path must differ from merge input {}",
            file_name_or_path(conflict)
        ))
    } else {
        Ok(())
    }
}

pub(super) fn set_merge_ui_failure(app: &App, message: &str) {
    app.set_merge_job_active(false);
    app.set_merge_progress(0.0);
    app.set_merge_status_text("Failed".into());
    app.set_merge_detail_text(message.into());
}

pub(in super::super) fn default_modify_output_path(selected_midi_name: &str) -> PathBuf {
    let selected_path = PathBuf::from(selected_midi_name);
    let stem = selected_path
        .file_stem()
        .and_then(OsStr::to_str)
        .filter(|name| !name.is_empty())
        .unwrap_or("modified-output");
    let file_name = format!("{stem}-modified.mid");
    selected_path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .map(|parent| parent.join(file_name.clone()))
        .unwrap_or_else(|| PathBuf::from(file_name))
}

pub(super) fn normalize_modify_output_path(path: &std::path::Path) -> PathBuf {
    normalize_midi_output_path(path)
}

pub(super) fn current_modify_output_path(app: &App) -> Result<PathBuf, String> {
    let raw = app.get_modify_output_path_text();
    if raw.is_empty() {
        return Err("choose an output path".into());
    }
    Ok(normalize_modify_output_path(std::path::Path::new(
        raw.as_str(),
    )))
}

pub(super) fn parse_modify_config(app: &App) -> Result<MidiFileProcessingConfig, String> {
    parse_modify_config_text(app.get_modify_config_text().as_str())
}

pub(super) fn set_modify_ui_failure(app: &App, message: &str) {
    app.set_modify_job_active(false);
    app.set_modify_progress(0.0);
    app.set_modify_status_text("Failed".into());
    app.set_modify_detail_text(message.into());
}

pub(in super::super) fn events_error_message(events: &[CoreEvent]) -> Option<String> {
    events.iter().find_map(|event| match event {
        CoreEvent::Error { message, .. } => Some(message.clone()),
        _ => None,
    })
}

pub(super) fn envelope_curve_from_label(label: &str) -> EnvelopeCurveType {
    match label {
        "linear" => EnvelopeCurveType::Linear,
        _ => EnvelopeCurveType::Exponential,
    }
}

pub(super) fn bool_from_label(label: &str) -> bool {
    matches!(label, "on" | "yes" | "true" | "1")
}

pub(super) fn ignore_range_from_keep_velocity(label: &str) -> Option<std::ops::RangeInclusive<u8>> {
    let trimmed = label.trim();
    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("off") {
        return Some(0..=0);
    }

    let keep_from = trimmed.parse::<u8>().ok()?.clamp(1, 127);
    if keep_from <= 1 {
        Some(0..=0)
    } else {
        Some(1..=keep_from - 1)
    }
}

pub(super) fn normalize_top_bar_color(value: &str) -> Option<String> {
    PfaKeyboardProjectorConfig::normalize_top_bar_color(value)
}

pub(super) fn palette_mut(scene: &mut SceneConfig) -> Option<&mut NotePaletteConfig> {
    match scene {
        SceneConfig::TwoD(config) => match &mut config.notes {
            NoteProjectorConfig::Flat(notes) => Some(&mut notes.palette),
            NoteProjectorConfig::Pfa(notes) => Some(&mut notes.palette),
        },
        SceneConfig::ThreeD(ThreeDSceneConfig::PianoTrailClassic(config)) => {
            Some(&mut config.palette)
        }
    }
}

pub(super) fn background_mut(scene: &mut SceneConfig) -> &mut ProjectorBackgroundConfig {
    scene.background_mut()
}

pub(super) fn ptc_mut(scene: &mut SceneConfig) -> Option<&mut PianoTrailClassicSceneConfig> {
    match scene {
        SceneConfig::ThreeD(ThreeDSceneConfig::PianoTrailClassic(config)) => Some(config),
        _ => None,
    }
}

pub(super) fn update_pfa_note_bool(
    app: &App,
    bridge: &UiCoreBridge,
    shared_state: &Arc<Mutex<UiViewModel>>,
    value: &str,
    mutate: impl Fn(&mut meridian_core::render::PfaNoteProjectorConfig, bool) + 'static,
) {
    let enabled = bool_from_label(value);
    update_video_scene(app, bridge, shared_state, move |scene| {
        if let SceneConfig::TwoD(config) = scene {
            if let NoteProjectorConfig::Pfa(notes) = &mut config.notes {
                mutate(notes, enabled);
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modify_output_guard_rejects_same_path_after_normalization() {
        let selected = PathBuf::from("fixtures/input.mid");
        let output = PathBuf::from("fixtures/input");

        let error = validate_modify_job_output_path(&selected, &output).unwrap_err();

        assert!(error.contains("selected input MIDI"));
    }

    #[test]
    fn modify_output_guard_rejects_relative_and_absolute_same_path() {
        let relative = PathBuf::from("fixtures/shared.mid");
        let absolute = std::env::current_dir().unwrap().join(&relative);

        assert!(validate_modify_job_output_path(&absolute, &relative).is_err());
    }

    #[test]
    fn merge_output_guard_rejects_any_matching_input() {
        let inputs = vec![
            PathBuf::from("fixtures/first.mid"),
            PathBuf::from("fixtures/second.midi"),
        ];
        let output = PathBuf::from("fixtures/second.midi");

        let error = validate_merge_job_output_path(&inputs, &output).unwrap_err();

        assert!(error.contains("merge input"));
        assert!(error.contains("second.midi"));
    }

    #[test]
    fn merge_output_guard_allows_distinct_output() {
        let inputs = vec![PathBuf::from("fixtures/first.mid")];
        let output = PathBuf::from("fixtures/merged.mid");

        assert!(validate_merge_job_output_path(&inputs, &output).is_ok());
    }
}

pub(super) fn update_pfa_keyboard_bool(
    app: &App,
    bridge: &UiCoreBridge,
    shared_state: &Arc<Mutex<UiViewModel>>,
    value: &str,
    mutate: impl Fn(&mut meridian_core::render::PfaKeyboardProjectorConfig, bool) + 'static,
) {
    let enabled = bool_from_label(value);
    update_video_scene(app, bridge, shared_state, move |scene| {
        if let SceneConfig::TwoD(config) = scene {
            if let KeyboardProjectorConfig::Pfa(keyboard) = &mut config.keyboard {
                mutate(keyboard, enabled);
            }
        }
    });
}

pub(super) fn update_ptc_bool(
    app: &App,
    bridge: &UiCoreBridge,
    shared_state: &Arc<Mutex<UiViewModel>>,
    value: &str,
    mutate: impl Fn(&mut PianoTrailClassicSceneConfig, bool) + 'static,
) {
    let enabled = bool_from_label(value);
    update_video_scene(app, bridge, shared_state, move |scene| {
        if let Some(config) = ptc_mut(scene) {
            mutate(config, enabled);
        }
    });
}

pub(super) fn update_ptc_f32(
    app: &App,
    bridge: &UiCoreBridge,
    shared_state: &Arc<Mutex<UiViewModel>>,
    value: &str,
    mutate: impl Fn(&mut PianoTrailClassicSceneConfig, f32) + 'static,
) {
    let Ok(parsed) = value.parse::<f32>() else {
        return;
    };
    update_video_scene(app, bridge, shared_state, move |scene| {
        if let Some(config) = ptc_mut(scene) {
            mutate(config, parsed);
        }
    });
}
