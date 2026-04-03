use crate::midi::{DisplacedMIDINote, MIDIColorPair, MIDINoteColumnView};

#[derive(Clone, Copy, Debug)]
pub(crate) struct VisibleNoteSpan {
    pub start: f32,
    pub end: f32,
    pub unclamped_end: f32,
    pub active: bool,
    pub has_ended: bool,
    pub color: MIDIColorPair,
}

pub(crate) fn for_each_visible_note<C, F>(
    column: C,
    render_start: f32,
    render_end: f32,
    clamp_start_to_zero: bool,
    clamp_end_to_zero: bool,
    mut callback: F,
) -> usize
where
    C: MIDINoteColumnView,
    F: FnMut(VisibleNoteSpan),
{
    let mut visible = 0usize;
    for note in column.iterate_displaced_notes() {
        if let Some(note) = clip_visible_note(
            note,
            render_start,
            render_end,
            clamp_start_to_zero,
            clamp_end_to_zero,
        ) {
            visible += 1;
            callback(note);
        }
    }
    visible
}

fn clip_visible_note(
    note: DisplacedMIDINote,
    render_start: f32,
    render_end: f32,
    clamp_start_to_zero: bool,
    clamp_end_to_zero: bool,
) -> Option<VisibleNoteSpan> {
    let start = note.start;
    let unclamped_end = start + note.len;
    let mut end = unclamped_end;
    if end < render_start || start >= render_end {
        return None;
    }

    let mut clipped_start = start.max(render_start);
    if clamp_start_to_zero {
        clipped_start = clipped_start.max(0.0);
    }
    if clamp_end_to_zero {
        end = end.max(0.0);
    }

    Some(VisibleNoteSpan {
        start: clipped_start,
        end: end.min(render_end),
        unclamped_end,
        active: start <= 0.0 && end > 0.0,
        has_ended: unclamped_end <= render_end,
        color: note.color,
    })
}
