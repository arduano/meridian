pub mod flat;
pub mod headless;
pub mod miditrail;
pub mod pfa;
pub mod shared;

pub use shared::{
    FlatKeyboardProjectorConfig, FlatNoteProjectorConfig, KeyboardHeightSpec,
    KeyboardProjectorConfig, MiditrailSceneConfig, NotePaletteConfig, NoteProjectorConfig,
    PfaKeyboardProjectorConfig, PfaNoteProjectorConfig, PfaTopColor, ProjectedScene,
    ProjectorImageConfig, RendererKind, SceneConfig, SceneLayer, SceneLayout, ScenePhysicsState,
    SceneQuad, ThreeDSceneConfig, TwoDSceneConfig, ZenithPaletteSpec, tick_scene_physics,
};

use crate::midi::{backend::MIDIFileUnion, views::MIDIFileViewsUnion};
use flat::{FlatKeyboardProjector, FlatNoteProjector};
use miditrail::project_miditrail_scene;
use pfa::{PfaKeyboardProjector, PfaNoteProjector};
use shared::{
    KeyboardProjectorConfig as KeyboardConfig, NoteProjectorConfig as NoteConfig, SceneConfig::*,
};

pub fn project_scene(
    midi: &mut MIDIFileUnion,
    current_time: f64,
    physics: Option<&ScenePhysicsState>,
    layout: &SceneLayout,
) -> ProjectedScene {
    let mut scene = ProjectedScene::default();
    let views = midi.get_current_column_views(current_time, layout.view_range);
    project_scene_views_into(&views, physics, layout, &mut scene);
    scene
}

pub fn project_scene_into(
    midi: &mut MIDIFileUnion,
    current_time: f64,
    physics: Option<&ScenePhysicsState>,
    layout: &SceneLayout,
    scene: &mut ProjectedScene,
) {
    let views = midi.get_current_column_views(current_time, layout.view_range);
    project_scene_views_into(&views, physics, layout, scene);
}

pub fn project_scene_views(
    views: &MIDIFileViewsUnion<'_>,
    physics: Option<&ScenePhysicsState>,
    layout: &SceneLayout,
) -> ProjectedScene {
    let mut scene = ProjectedScene::default();
    project_scene_views_into(views, physics, layout, &mut scene);
    scene
}

pub fn project_scene_views_into(
    views: &MIDIFileViewsUnion<'_>,
    physics: Option<&ScenePhysicsState>,
    layout: &SceneLayout,
    scene: &mut ProjectedScene,
) {
    scene.clear();
    let piano_height = layout.piano_height();
    match &layout.scene {
        TwoD(config) => {
            match &config.notes {
                NoteConfig::Flat(config) => FlatNoteProjector(config.clone()).project_notes(
                    views,
                    layout,
                    piano_height,
                    scene,
                ),
                NoteConfig::Pfa(config) => PfaNoteProjector(config.clone()).project_notes(
                    views,
                    layout,
                    piano_height,
                    scene,
                ),
            }

            match &config.keyboard {
                KeyboardConfig::Flat(config) => {
                    FlatKeyboardProjector(*config).project_keyboard(layout, piano_height, scene)
                }
                KeyboardConfig::Pfa(config) => {
                    PfaKeyboardProjector(*config).project_keyboard(layout, piano_height, scene)
                }
            }
        }
        ThreeD(ThreeDSceneConfig::Miditrail(config)) => {
            project_miditrail_scene(config, physics, views, layout, scene)
        }
    }
}

pub(crate) trait NoteProjector {
    fn project_notes(
        &self,
        views: &MIDIFileViewsUnion<'_>,
        layout: &SceneLayout,
        piano_height: f32,
        scene: &mut ProjectedScene,
    );
}

pub(crate) trait KeyboardProjector {
    fn project_keyboard(&self, layout: &SceneLayout, piano_height: f32, scene: &mut ProjectedScene);
}
