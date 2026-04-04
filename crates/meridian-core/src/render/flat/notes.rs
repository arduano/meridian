use crate::midi::{
    MIDINoteColumnView, MIDINoteViews, ram::view::InRamCurrentNoteViews, views::MIDIFileViewsUnion,
};

use super::super::{
    SceneLayout,
    shared::{
        FlatNoteProjectorConfig, KeyActivity, NoteInstance, NoteShaderKind, ProjectedScene,
        SceneLayer, build_key_x_layout, for_each_visible_note, is_black_key, linear_key_span,
        normalized_key_range,
    },
};

pub(crate) fn project_flat_notes(
    _config: &FlatNoteProjectorConfig,
    views: &MIDIFileViewsUnion<'_>,
    layout: &SceneLayout,
    piano_height: f32,
    scene: &mut ProjectedScene,
) {
    scene.set_note_shader_kind(NoteShaderKind::Flat);
    scene.notes_black_first = false;
    let view_range = views.range().length() as f32;
    let (first_key, last_key_exclusive) = normalized_key_range(layout.first_key, layout.last_key);
    let key_layout = build_key_x_layout(
        first_key,
        last_key_exclusive,
        true,
        crate::render::shared::PFA_BLACK_KEY_PROFILE,
    );
    let (_, last_key, _) = linear_key_span(first_key, last_key_exclusive);
    scene.set_note_params(
        piano_height,
        (1.0 - piano_height) / view_range.max(0.001),
        0.0,
        0.0,
    );
    for key in first_key..=last_key {
        scene.set_note_key_x(
            key as u8,
            key_layout.x1[key],
            key_layout.x1[key] + key_layout.width[key],
        );
    }

    match views {
        MIDIFileViewsUnion::InRam(views) => {
            project_flat_notes_in_ram(views, first_key, last_key, view_range, scene)
        }
    }
}

fn project_flat_notes_in_ram(
    views: &InRamCurrentNoteViews<'_>,
    first_key: usize,
    last_key: usize,
    view_range: f32,
    scene: &mut ProjectedScene,
) {
    for key in first_key..=last_key {
        let is_black = is_black_key(key as u8);
        let note_layer = if is_black {
            SceneLayer::BlackNotes
        } else {
            SceneLayer::WhiteNotes
        };
        let column = views.get_column(key);
        let mut activity = KeyActivity::default();

        scene.visible_notes += for_each_visible_note(
            column.iterate_displaced_notes(),
            0.0,
            view_range,
            false,
            false,
            |note| {
                let average = note.color.average();
                let color = average.to_rgba(if is_black { 0.94 } else { 0.88 });
                scene.push_note_layer(
                    note_layer,
                    NoteInstance::new(
                        key as u32,
                        note.start,
                        note.end,
                        average.to_rgba_packed(if is_black { 240 } else { 224 }),
                        average.to_rgba_packed(if is_black { 240 } else { 224 }),
                    ),
                );

                if note.active {
                    activity.pressed = true;
                    activity.left = color;
                    activity.right = color;
                }
                scene.note_quads += 1;
            },
        );

        if activity.pressed {
            scene.active_keys += 1;
        }
        scene.set_key_activity(key, activity);
    }
}
