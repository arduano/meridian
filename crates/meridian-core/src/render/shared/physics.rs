use crate::midi::{backend::MIDIFileUnion, views::MIDIFileViewsUnion};

use super::{SceneConfig, SceneLayout, ThreeDSceneConfig};
use crate::render::miditrail::{MiditrailPhysicsState, tick_miditrail_physics};

#[derive(Clone, Debug)]
pub enum ScenePhysicsState {
    TwoD,
    Miditrail(MiditrailPhysicsState),
}

impl ScenePhysicsState {
    pub fn new(scene: &SceneConfig) -> Self {
        match scene {
            SceneConfig::TwoD(_) => Self::TwoD,
            SceneConfig::ThreeD(ThreeDSceneConfig::Miditrail(_)) => {
                Self::Miditrail(MiditrailPhysicsState::default())
            }
        }
    }

    pub fn reset(&mut self, scene: &SceneConfig) {
        *self = Self::new(scene);
    }
}

pub fn tick_scene_physics(
    midi: &mut MIDIFileUnion,
    current_time: f64,
    layout: &SceneLayout,
    physics: &mut ScenePhysicsState,
    delta_seconds: f64,
) {
    let delta_seconds = delta_seconds.max(0.0);
    let views = midi.get_current_column_views(current_time, layout.view_range);
    tick_scene_physics_views(&views, layout, physics, delta_seconds);
}

pub fn tick_scene_physics_views(
    views: &MIDIFileViewsUnion<'_>,
    layout: &SceneLayout,
    physics: &mut ScenePhysicsState,
    delta_seconds: f64,
) {
    if delta_seconds <= 0.0 {
        return;
    }
    match (&layout.scene, physics) {
        (SceneConfig::TwoD(_), ScenePhysicsState::TwoD) => {}
        (
            SceneConfig::ThreeD(ThreeDSceneConfig::Miditrail(config)),
            ScenePhysicsState::Miditrail(state),
        ) => tick_miditrail_physics(state, config, views, delta_seconds as f32),
        (scene, state) => state.reset(scene),
    }
}
