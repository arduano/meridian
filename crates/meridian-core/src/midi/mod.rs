//! MIDI domain boundary.
//!
//! This module is the public entry point for MIDI data, derived views, and
//! file-oriented helpers in `meridian-core`.
//!
//! The shape is intentionally split by responsibility:
//! - `types` and `traits` own the public data model and view interfaces
//! - `parsed`, `processed`, `materialized`, and `display_cache` own concrete data paths
//! - `file_support`, `file_processing`, `file_merge`, and `inspect` cover file-oriented workflows
//! - `analysis` and `modifier_tools` hold the heavier derived-data logic
//!
//! If you are looking for a specific behavior, start by deciding whether it is
//! a domain type, a file workflow, or a derived-data pipeline.

#[path = "analysis/mod.rs"]
pub mod analysis;
pub mod audio_cache;
pub mod backend;
pub mod cache;
pub mod colors;
pub mod display_cache;
pub mod file_merge;
mod file_support;
pub mod file_processing;
pub mod inspect;
mod materialized;
pub mod modifier_tools;
pub mod parsed;
pub mod processed;
pub mod processing;
pub mod ram;
mod traits;
mod types;
mod union;
pub mod tempo_map;
#[cfg(test)]
pub(crate) mod test_support;
pub mod tools;
mod traversal;
pub mod views;

pub use cache::MidiCacheStack;
pub use colors::{MIDIColor, MIDIColorPair};
pub use file_merge::{MidiFilesMergeConfig, MidiFilesMergeMode};
pub use inspect::MidiFileInspection;
pub use traits::{MIDIFile, MIDIFileBase, MIDINoteColumnView, MIDINoteViews};
pub use modifier_tools::{
    ChannelMapEntry, ChannelRemapTool, KeyMapEntry, KeyMapTool, KeyRange, MetaTextTool, TextKind,
};
pub use processed::ProcessedMidi;
pub use processing::{
    EventFilterConfig, MidiFileProcessingConfig, MidiProcessingConfig, ZeroVelocityNoteOnMode,
};
pub use types::{
    DisplacedMIDINote, MIDIAnalysisSummary, MIDIFileStats, MIDIFileUniqueSignature, MIDIViewRange,
    MidiBuildProgress, MIDI_KEY_COUNT,
};
pub(crate) use types::TrackAndChannel;
pub use union::{MIDIFileUnion, MIDIFileViewsUnion, MIDINoteColumnViewUnion};
pub use tools::*;
pub(crate) use file_support::{cleanup_output_file, open_file_and_signature};
