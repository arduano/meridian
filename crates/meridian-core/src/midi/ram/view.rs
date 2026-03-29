use rayon::iter::{IntoParallelRefMutIterator, ParallelIterator};
use std::{
    ops::{Coroutine, CoroutineState},
    pin::Pin,
};

use crate::midi::{
    DisplacedMIDINote, MIDIAnalysisSummary, MIDIColorPair, MIDINoteColumnView, MIDINoteViews,
    MIDIViewRange,
};

use super::{cache::InRamMidiCache, column::InRamNoteColumn};

struct GenIter<G>(Pin<Box<G>>);

impl<T, G> Iterator for GenIter<G>
where
    G: Coroutine<Return = (), Yield = T>,
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        match self.0.as_mut().resume(()) {
            CoroutineState::Yielded(value) => Some(value),
            CoroutineState::Complete(()) => None,
        }
    }
}

pub struct InRamNoteViewData {
    columns: Vec<InRamNoteColumn>,
    default_track_colors: Vec<MIDIColorPair>,
    view_range: MIDIViewRange,
}

pub struct InRamCurrentNoteViews<'a> {
    data: &'a InRamNoteViewData,
}

impl<'a> InRamCurrentNoteViews<'a> {
    pub fn new(data: &'a InRamNoteViewData) -> Self {
        Self { data }
    }
}

impl InRamNoteViewData {
    pub fn new(columns: Vec<InRamNoteColumn>, colors: Vec<MIDIColorPair>) -> Self {
        Self {
            columns,
            default_track_colors: colors,
            view_range: MIDIViewRange::default(),
        }
    }

    pub fn from_cache(cache: &InRamMidiCache) -> Self {
        let columns = cache
            .columns()
            .iter()
            .cloned()
            .map(InRamNoteColumn::from_shared_blocks)
            .collect();
        Self::new(columns, cache.default_colors())
    }

    pub fn apply_default_track_colors(&mut self, colors: Vec<MIDIColorPair>) {
        self.default_track_colors = colors;
    }

    pub fn track_count(&self) -> usize {
        (self.default_track_colors.len() / 16).max(1)
    }

    pub fn passed_notes(&self) -> u64 {
        self.columns
            .iter()
            .map(|column| column.data.notes_to_keyboard)
            .sum()
    }

    pub fn analysis_summary(&self) -> MIDIAnalysisSummary {
        let mut total_blocks = 0_u64;
        let mut keys_with_notes = 0_usize;
        let mut max_blocks_per_key = 0_usize;
        let mut max_notes_in_block = 0_usize;
        let mut densest_key = 0_usize;
        let mut densest_key_notes = 0_u64;

        for (key, column) in self.columns.iter().enumerate() {
            let block_count = column.blocks.len();
            let note_count = column
                .blocks
                .iter()
                .map(|block| block.notes.len() as u64)
                .sum::<u64>();

            total_blocks += block_count as u64;
            if note_count > 0 {
                keys_with_notes += 1;
            }
            max_blocks_per_key = max_blocks_per_key.max(block_count);

            if note_count > densest_key_notes {
                densest_key = key;
                densest_key_notes = note_count;
            }

            for block in column.blocks.iter() {
                max_notes_in_block = max_notes_in_block.max(block.notes.len());
            }
        }

        MIDIAnalysisSummary {
            total_blocks,
            keys_with_notes,
            max_blocks_per_key,
            max_notes_in_block,
            densest_key,
            densest_key_notes,
        }
    }

    pub fn key_note_counts(&self) -> [u64; crate::midi::MIDI_KEY_COUNT] {
        let mut counts = [0_u64; crate::midi::MIDI_KEY_COUNT];
        for (key, column) in self.columns.iter().enumerate() {
            counts[key] = column
                .blocks
                .iter()
                .map(|block| block.notes.len() as u64)
                .sum();
        }
        counts
    }

    pub fn shift_view_range(&mut self, new_view_range: MIDIViewRange) {
        let old_view_range = self.view_range;
        self.view_range = new_view_range;

        self.columns.par_iter_mut().for_each(|column| {
            if column.blocks.is_empty() {
                return;
            }

            let blocks = &column.blocks;
            let data = &mut column.data;

            let mut new_block_start = data.block_range.start;
            let mut new_block_end = data.block_range.end;

            if new_view_range.end > old_view_range.end {
                while new_block_end < blocks.len() {
                    let block = &blocks[new_block_end];
                    if block.start >= new_view_range.end {
                        break;
                    }
                    data.notes_to_render_end += block.notes.len() as u64;
                    new_block_end += 1;
                }
            } else if new_view_range.end < old_view_range.end {
                while new_block_end > 0 {
                    let block = &blocks[new_block_end - 1];
                    if block.start < new_view_range.end {
                        break;
                    }
                    data.notes_to_render_end -= block.notes.len() as u64;
                    new_block_end -= 1;
                }
            } else {
                // No change in view end.
            }

            if new_view_range.start > old_view_range.start {
                while new_block_start < blocks.len() {
                    let block = &blocks[new_block_start];
                    if block.max_end() >= new_view_range.start {
                        break;
                    }
                    data.notes_to_render_start += block.notes.len() as u64;
                    new_block_start += 1;
                }

                while data.blocks_to_keyboard < blocks.len() {
                    let block = &blocks[data.blocks_to_keyboard];
                    if block.start > new_view_range.start {
                        break;
                    }
                    data.notes_to_keyboard += block.notes.len() as u64;
                    data.blocks_to_keyboard += 1;
                }
            } else if new_view_range.start < old_view_range.start {
                // Rebuild the start-side counters when seeking backward.
                data.notes_to_render_start = 0;
                new_block_start = 0;
                data.notes_to_keyboard = 0;
                data.blocks_to_keyboard = 0;

                while new_block_start < blocks.len() {
                    let block = &blocks[new_block_start];
                    if block.max_end() >= new_view_range.start {
                        break;
                    }
                    data.notes_to_render_start += block.notes.len() as u64;
                    new_block_start += 1;
                    data.notes_to_keyboard += block.notes.len() as u64;
                    data.blocks_to_keyboard += 1;
                }

                while data.blocks_to_keyboard < blocks.len() {
                    let block = &blocks[data.blocks_to_keyboard];
                    if block.start > new_view_range.start {
                        break;
                    }
                    data.notes_to_keyboard += block.notes.len() as u64;
                    data.blocks_to_keyboard += 1;
                }
            } else {
                // No change in view start.
            }

            data.block_range = new_block_start..new_block_end;
        });
    }
}

impl<'a> MIDINoteViews for InRamCurrentNoteViews<'a> {
    type View<'b>
        = InRamNoteColumnView<'b>
    where
        Self: 'a + 'b;

    fn get_column(&self, key: usize) -> Self::View<'_> {
        InRamNoteColumnView {
            view: self.data,
            column: &self.data.columns[key],
            view_range: self.data.view_range,
        }
    }

    fn range(&self) -> MIDIViewRange {
        self.data.view_range
    }
}

pub struct InRamNoteColumnView<'a> {
    view: &'a InRamNoteViewData,
    column: &'a InRamNoteColumn,
    view_range: MIDIViewRange,
}

struct InRamNoteBlockIter<'a, Iter: Iterator<Item = DisplacedMIDINote>> {
    view: &'a InRamNoteColumnView<'a>,
    iter: Iter,
}

impl<'a> MIDINoteColumnView for InRamNoteColumnView<'a> {
    type Iter<'b> = impl 'b + ExactSizeIterator<Item = DisplacedMIDINote> where Self: 'b;

    fn iterate_displaced_notes(&self) -> Self::Iter<'_> {
        let colors = &self.view.default_track_colors;
        let iter = GenIter(Box::pin(#[coroutine] move || {
            for block_index in self.column.data.block_range.clone().rev() {
                let block = &self.column.blocks[block_index];
                let start = (block.start - self.view_range.start) as f32;

                for note in block.notes.iter().rev() {
                    yield DisplacedMIDINote {
                        start,
                        len: note.len,
                        color: note
                            .explicit_colors
                            .unwrap_or(colors[note.track_chan.as_usize()]),
                    };
                }
            }
        }));

        InRamNoteBlockIter { view: self, iter }
    }
}

impl<Iter: Iterator<Item = DisplacedMIDINote>> Iterator for InRamNoteBlockIter<'_, Iter> {
    type Item = DisplacedMIDINote;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }
}

impl<Iter: Iterator<Item = DisplacedMIDINote>> ExactSizeIterator for InRamNoteBlockIter<'_, Iter> {
    fn len(&self) -> usize {
        let data = &self.view.column.data;
        (data.notes_to_render_end - data.notes_to_render_start) as usize
    }
}

impl Drop for InRamNoteViewData {
    fn drop(&mut self) {
        let data = std::mem::take(&mut self.columns);
        std::thread::spawn(move || drop(data));
    }
}
