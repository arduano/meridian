use std::ops::Range;

use super::block::InRamNoteBlock;

pub struct InRamNoteColumnViewData {
    pub notes_to_render_start: u64,
    pub notes_to_render_end: u64,
    pub block_range: Range<usize>,
    pub notes_to_keyboard: u64,
    pub blocks_to_keyboard: usize,
}

impl InRamNoteColumnViewData {
    pub fn new() -> Self {
        Self {
            notes_to_render_start: 0,
            notes_to_render_end: 0,
            block_range: 0..0,
            notes_to_keyboard: 0,
            blocks_to_keyboard: 0,
        }
    }
}

pub struct InRamNoteColumn {
    pub data: InRamNoteColumnViewData,
    pub blocks: Vec<InRamNoteBlock>,
}

impl InRamNoteColumn {
    pub fn new(blocks: Vec<InRamNoteBlock>) -> Self {
        Self {
            blocks,
            data: InRamNoteColumnViewData::new(),
        }
    }
}
