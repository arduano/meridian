//! Core snapshot/restore helpers for video rendering.
//!
//! This module keeps the render orchestration code from having to know how to
//! read the active core state or how to replay it after a render exits.

use crate::{
    CoreHandle, MeridianError,
    protocol::{CoreCommand, CoreEvent, StateSnapshot},
};

pub(crate) fn read_core_state(core: &CoreHandle) -> Result<StateSnapshot, MeridianError> {
    let events = core.request(CoreCommand::GetState)?;
    match events.into_iter().next() {
        Some(CoreEvent::StateSnapshot { state }) => Ok(state),
        _ => Err(MeridianError::Protocol(
            "unexpected response while reading core state".into(),
        )),
    }
}

pub(crate) fn restore_core_state(
    core: &CoreHandle,
    state: &StateSnapshot,
) -> Result<(), MeridianError> {
    if let Some(path) = &state.midi_path {
        core.request(CoreCommand::LoadMidi { path: path.clone() })?;
    } else {
        core.request(CoreCommand::UnloadRenderContext)?;
    }
    core.request(CoreCommand::SetSceneConfig {
        scene: state.scene.clone(),
    })?;
    core.request(CoreCommand::SetViewRange {
        seconds: state.view_range,
        time_space: Some(state.time_space),
    })?;
    core.request(CoreCommand::SetKeyRange {
        first_key: state.first_key,
        last_key: state.last_key,
    })?;
    core.request(CoreCommand::SetViewport {
        width: state.viewport_width,
        height: state.viewport_height,
    })?;
    core.request(CoreCommand::SetTime {
        time: state.current_time,
    })?;
    core.request(CoreCommand::SetPlaying {
        playing: state.playing,
    })?;
    Ok(())
}
