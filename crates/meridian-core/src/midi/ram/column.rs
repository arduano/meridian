use std::ops::Range;
use std::sync::Arc;

use super::block::InRamNoteBlock;

pub struct InRamNoteColumnViewData {
    /// Number of notes from the beginning of the midi to the end of the render view.
    pub notes_to_render_end: u64,
    /// Number of notes from the beginning of the midi to the start of the render view.
    pub notes_to_render_start: u64,
    /// The range of blocks that are in the current render view.
    pub block_range: Range<usize>,
    /// Number of notes that have passed the keyboard.
    pub notes_to_keyboard: u64,
    /// Number of blocks that have passed the keyboard.
    pub blocks_to_keyboard: usize,
}

impl Default for InRamNoteColumnViewData {
    fn default() -> Self {
        Self::new()
    }
}

impl InRamNoteColumnViewData {
    pub fn new() -> Self {
        Self {
            notes_to_render_end: 0,
            notes_to_render_start: 0,
            block_range: 0..0,
            notes_to_keyboard: 0,
            blocks_to_keyboard: 0,
        }
    }
}

pub struct InRamNoteColumn {
    pub data: InRamNoteColumnViewData,
    pub blocks: Arc<[InRamNoteBlock]>,
}

impl InRamNoteColumn {
    pub fn new(blocks: Vec<InRamNoteBlock>) -> Self {
        Self {
            blocks: blocks.into(),
            data: InRamNoteColumnViewData::new(),
        }
    }

    pub fn from_shared_blocks(blocks: Arc<[InRamNoteBlock]>) -> Self {
        Self {
            blocks,
            data: InRamNoteColumnViewData::new(),
        }
    }
}
