pub mod export;
pub mod flat;
pub mod headless;
pub mod pfa;
pub mod piano_trail_classic;
pub mod shared;
pub mod text;

pub use shared::{
    tick_scene_physics, DisplayTimeSpace, FlatKeyboardProjectorConfig, FlatNoteProjectorConfig,
    KeyboardHeightSpec, KeyboardProjectorConfig, NotePaletteConfig, NoteProjectorConfig,
    PfaKeyboardProjectorConfig, PfaNoteProjectorConfig, PianoTrailClassicSceneConfig,
    ProjectedScene, ProjectorBackgroundConfig, ProjectorBackgroundScalingMode,
    ProjectorImageConfig, RendererKind, SceneConfig, SceneLayer, SceneLayout, ScenePhysicsState,
    SceneQuad, TextAlignment, TextAnchor, TextOverlayConfig, TextRowConfig, TextSceneConfig,
    TextStyleConfig, TextValueFormat, TextValueSource, ThreeDSceneConfig, TwoDSceneConfig,
    ZenithPaletteSpec, PFA_BLUE_TOP_BAR_COLOR, PFA_GREEN_TOP_BAR_COLOR, PFA_RED_TOP_BAR_COLOR,
};

use crate::midi::{backend::MIDIFileUnion, views::MIDIFileViewsUnion};
use flat::{project_flat_keyboard, project_flat_notes};
use pfa::{project_pfa_keyboard, project_pfa_notes};
use piano_trail_classic::project_piano_trail_classic_scene;
use shared::{
    KeyboardProjectorConfig as KeyboardConfig, NoteProjectorConfig as NoteConfig, SceneConfig::*,
};
use text::project_text_scene;

pub fn project_scene(
    midi: &mut MIDIFileUnion,
    current_time: f64,
    physics: Option<&ScenePhysicsState>,
    layout: &SceneLayout,
) -> ProjectedScene {
    let mut scene = ProjectedScene::default();
    let views = midi.get_current_column_views(current_time, layout.view_range, layout.time_space);
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
    let views = midi.get_current_column_views(current_time, layout.view_range, layout.time_space);
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
                NoteConfig::Flat(config) => {
                    project_flat_notes(config, views, layout, piano_height, scene)
                }
                NoteConfig::Pfa(config) => {
                    project_pfa_notes(config, views, layout, piano_height, scene)
                }
            }

            match &config.keyboard {
                KeyboardConfig::Flat(_config) => project_flat_keyboard(layout, piano_height, scene),
                KeyboardConfig::Pfa(config) => {
                    project_pfa_keyboard(config, layout, piano_height, scene)
                }
            }
        }
        ThreeD(ThreeDSceneConfig::PianoTrailClassic(config)) => {
            project_piano_trail_classic_scene(config, physics, views, layout, scene)
        }
        Text(config) => project_text_scene(
            config,
            layout,
            &text::TextRenderMetrics {
                viewport_width: layout.viewport_width,
                viewport_height: layout.viewport_height,
                ..text::TextRenderMetrics::default()
            },
            scene,
        ),
    }
}
