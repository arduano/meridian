use std::{
    env,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use meridian_core::{
    audio::{AudioBackend, AudioConfig},
    protocol::{CoreEvent, StateSnapshot},
    render::{DisplayTimeSpace, RendererKind, SceneConfig, SceneLayout},
};

use super::{view::App, view_model::UiViewModel};

mod analysis;
mod apply;
mod audio;
mod formatting;
mod merge;
mod modify;
mod reduce;
mod video;

#[derive(Debug, Clone)]
pub struct UiOptions {
    pub midi_path: Option<PathBuf>,
    pub renderer: Option<RendererKind>,
    pub start_time: Option<f64>,
    pub view_range: Option<f64>,
    pub first_key: Option<u8>,
    pub last_key: Option<u8>,
    pub disable_wgpu: bool,
}

impl Default for UiOptions {
    fn default() -> Self {
        Self {
            midi_path: None,
            renderer: None,
            start_time: None,
            view_range: None,
            first_key: None,
            last_key: None,
            disable_wgpu: matches!(
                env::var("MERIDIAN_DISABLE_WGPU").as_deref(),
                Ok("1" | "true" | "yes")
            ),
        }
    }
}

#[derive(Debug, Clone)]
pub struct UiStartupOptions {
    pub midi_path: Option<PathBuf>,
    pub audio: AudioConfig,
    pub scene: SceneConfig,
    pub start_time: f64,
    pub view_range: f64,
    pub time_space: DisplayTimeSpace,
    pub first_key: u8,
    pub last_key: u8,
    pub disable_wgpu: bool,
}

impl Default for UiStartupOptions {
    fn default() -> Self {
        Self {
            midi_path: None,
            audio: AudioConfig {
                backend: AudioBackend::Xsynth,
                ..AudioConfig::default()
            },
            scene: SceneLayout::default().scene,
            start_time: 0.0,
            view_range: 0.5,
            time_space: DisplayTimeSpace::Time,
            first_key: 0,
            last_key: 127,
            disable_wgpu: matches!(
                env::var("MERIDIAN_DISABLE_WGPU").as_deref(),
                Ok("1" | "true" | "yes")
            ),
        }
    }
}

#[derive(Clone)]
pub struct UiFrameUpdate {
    pub state: StateSnapshot,
    pub visible_notes: usize,
    pub active_keys: usize,
    pub fps_text: String,
}

pub(super) fn app_has_active_midi_load(app: &App) -> bool {
    apply::app_has_active_midi_load(app)
}

pub fn apply_events_to_app(
    app: &App,
    shared_state: &Arc<Mutex<UiViewModel>>,
    events: &[meridian_core::protocol::CoreEvent],
) {
    apply::apply_events_to_app(app, shared_state, events);
}

pub fn reduce_core_events(shared_state: &Arc<Mutex<UiViewModel>>, events: &[CoreEvent]) {
    reduce::reduce_core_events(shared_state, events);
}

pub fn apply_frame_update_to_app(
    app: &App,
    shared_state: &Arc<Mutex<UiViewModel>>,
    update: &UiFrameUpdate,
) {
    apply::apply_frame_update_to_app(app, shared_state, update);
}

pub fn apply_merge_sources_to_app(app: &App, shared_state: &Arc<Mutex<UiViewModel>>) {
    merge::apply_merge_sources_to_app(app, shared_state);
}
