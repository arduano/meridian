use crate::midi::views::MIDIFileViewsUnion;

use super::{
    super::{
        NoteProjector, SceneLayout,
        shared::{KeyActivity, ProjectedScene, SceneLayer, is_black_key, solid_quad},
    },
    FlatNoteProjector, key_span,
};

impl NoteProjector for FlatNoteProjector {
    fn project_notes(
        &self,
        views: &MIDIFileViewsUnion<'_>,
        layout: &SceneLayout,
        piano_height: f32,
        scene: &mut ProjectedScene,
    ) {
        let _ = self.0;
        scene.notes_black_first = false;
        let (first_key, last_key, key_width) = key_span(layout);
        let view_range = views.range().length() as f32;

        for key in first_key..=last_key {
            let x1 = (key - first_key) as f32 * key_width;
            let x2 = x1 + key_width;
            let is_black = is_black_key(key as u8);
            let note_layer = if is_black {
                SceneLayer::BlackNotes
            } else {
                SceneLayer::WhiteNotes
            };
            let column = views.get_column(key);
            let mut activity = KeyActivity::default();

            column.for_each_displaced_note(|note| {
                let end = note.start + note.len;
                if end <= 0.0 || note.start >= view_range {
                    return;
                }

                let bottom =
                    piano_height + (note.start.max(0.0) / view_range) * (1.0 - piano_height);
                let top = piano_height + (end.min(view_range) / view_range) * (1.0 - piano_height);
                let color = note.color.to_rgba(if is_black { 0.94 } else { 0.88 });

                scene.push_quad(note_layer, solid_quad(x1, bottom, x2, top, color));

                if note.start <= 0.0 && end > 0.0 {
                    activity.pressed = true;
                    activity.left = color;
                    activity.right = color;
                }
                scene.visible_notes += 1;
                scene.note_quads += 1;
            });

            if activity.pressed {
                scene.active_keys += 1;
            }
            scene.set_key_activity(key, activity);
        }
    }
}
