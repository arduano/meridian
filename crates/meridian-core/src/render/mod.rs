pub mod pfa;
pub mod shared;

pub use shared::{ProjectedScene, RendererKind, SceneLayer, SceneLayout, SceneQuad};

use crate::midi::{MIDI_KEY_COUNT, backend::MIDIFileUnion, views::MIDIFileViewsUnion};
use pfa::PfaProjector;
use shared::{is_black_key, mix, shade, vertical_gradient_quad};

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
    match layout.renderer {
        RendererKind::Basic => BasicProjector.project_into(views, layout, scene),
        RendererKind::Pfa => PfaProjector::default().project_into(views, layout, scene),
    }
}

pub(crate) trait SceneProjector {
    fn project_into(
        &self,
        views: &MIDIFileViewsUnion<'_>,
        layout: &SceneLayout,
        scene: &mut ProjectedScene,
    );
}

struct BasicProjector;

impl SceneProjector for BasicProjector {
    fn project_into(
        &self,
        views: &MIDIFileViewsUnion<'_>,
        layout: &SceneLayout,
        scene: &mut ProjectedScene,
    ) {
        scene.notes_black_first = false;
        let first_key = layout.first_key.min(layout.last_key) as usize;
        let last_key = layout.last_key.max(layout.first_key) as usize;
        let view_range = views.range().length() as f32;
        let piano_height = layout.piano_height;
        let key_width = 1.0 / ((last_key - first_key + 1) as f32);

        for key in first_key..=last_key.min(MIDI_KEY_COUNT - 1) {
            let x1 = (key - first_key) as f32 * key_width;
            let x2 = x1 + key_width;
            let is_black = is_black_key(key as u8);
            let mut key_color = None;
            let column = views.get_column(key);

            column.for_each_displaced_note(|note| {
                let end = note.start + note.len;
                if end <= 0.0 || note.start >= view_range {
                    return;
                }

                let bottom =
                    piano_height + (note.start.max(0.0) / view_range) * (1.0 - piano_height);
                let top = piano_height + (end.min(view_range) / view_range) * (1.0 - piano_height);
                let color = note.color.to_rgba(if is_black { 0.94 } else { 0.88 });
                let border = shade(color, if is_black { 0.18 } else { -0.18 });
                let inset = key_width * if is_black { 0.10 } else { 0.06 };

                scene.push_quad(
                    if is_black {
                        SceneLayer::BlackNotes
                    } else {
                        SceneLayer::WhiteNotes
                    },
                    vertical_gradient_quad(x1, bottom, x2, top, border, border, border, border),
                );
                scene.push_quad(
                    if is_black {
                        SceneLayer::BlackNotes
                    } else {
                        SceneLayer::WhiteNotes
                    },
                    vertical_gradient_quad(
                        x1 + inset,
                        bottom + 0.003,
                        x2 - inset,
                        (top - 0.003).max(bottom + 0.003),
                        color,
                        color,
                        shade(color, 0.08),
                        shade(color, 0.08),
                    ),
                );

                if note.start <= 0.0 && end > 0.0 {
                    key_color = Some(color);
                }
                scene.visible_notes += 1;
                scene.note_quads += 2;
            });

            let key_top = if is_black {
                piano_height * 0.62
            } else {
                piano_height
            };
            let base = if is_black {
                [0.08, 0.16, 0.09, 1.0]
            } else {
                [0.74, 0.96, 0.76, 1.0]
            };
            let fill = key_color
                .map(|color| mix(base, color, if is_black { 0.75 } else { 0.45 }))
                .unwrap_or(base);
            let border = if is_black {
                shade(fill, 0.12)
            } else {
                shade(fill, -0.18)
            };
            if key_color.is_some() {
                scene.active_keys += 1;
            }

            scene.push_quad(
                if is_black {
                    SceneLayer::BlackKeys
                } else {
                    SceneLayer::WhiteKeys
                },
                vertical_gradient_quad(x1, 0.0, x2, key_top, border, border, border, border),
            );
            scene.push_quad(
                if is_black {
                    SceneLayer::BlackKeys
                } else {
                    SceneLayer::WhiteKeys
                },
                vertical_gradient_quad(
                    x1 + key_width * if is_black { 0.08 } else { 0.03 },
                    piano_height * 0.012,
                    x2 - key_width * if is_black { 0.08 } else { 0.03 },
                    key_top - piano_height * 0.012,
                    fill,
                    fill,
                    fill,
                    fill,
                ),
            );
            scene.keyboard_quads += 2;
        }
    }
}
