pub mod flat;
pub mod pfa;
pub mod shared;

pub use shared::{
    FlatKeyboardProjectorConfig, FlatNoteProjectorConfig, KeyboardHeightSpec,
    KeyboardProjectorConfig, NoteProjectorConfig, PfaKeyboardProjectorConfig,
    PfaNoteProjectorConfig, PfaTopColor, ProjectedScene, RendererKind, SceneConfig, SceneLayer,
    SceneLayout, SceneQuad, ThreeDSceneConfig, TwoDSceneConfig,
};

use crate::midi::{backend::MIDIFileUnion, views::MIDIFileViewsUnion};
use flat::{FlatKeyboardProjector, FlatNoteProjector};
use pfa::{PfaKeyboardProjector, PfaNoteProjector};
use shared::{
    KeyboardProjectorConfig as KeyboardConfig, NoteProjectorConfig as NoteConfig, SceneConfig::*,
};

pub fn project_scene(
    midi: &mut MIDIFileUnion,
    current_time: f64,
    layout: &SceneLayout,
) -> ProjectedScene {
    let mut scene = ProjectedScene::default();
    let views = midi.get_current_column_views(current_time, layout.view_range);
    project_scene_views_into(&views, layout, &mut scene);
    scene
}

pub fn project_scene_into(
    midi: &mut MIDIFileUnion,
    current_time: f64,
    layout: &SceneLayout,
    scene: &mut ProjectedScene,
) {
    let views = midi.get_current_column_views(current_time, layout.view_range);
    project_scene_views_into(&views, layout, scene);
}

pub fn project_scene_views(views: &MIDIFileViewsUnion<'_>, layout: &SceneLayout) -> ProjectedScene {
    let mut scene = ProjectedScene::default();
    project_scene_views_into(views, layout, &mut scene);
    scene
}

pub fn project_scene_views_into(
    views: &MIDIFileViewsUnion<'_>,
    layout: &SceneLayout,
    scene: &mut ProjectedScene,
) {
    scene.clear();
    let piano_height = layout.piano_height();
    match &layout.scene {
        TwoD(config) => {
            match &config.notes {
                NoteConfig::Flat(config) => {
                    FlatNoteProjector(*config).project_notes(views, layout, piano_height, scene)
                }
                NoteConfig::Pfa(config) => {
                    PfaNoteProjector(*config).project_notes(views, layout, piano_height, scene)
                }
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
        ThreeD(_) => {}
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
