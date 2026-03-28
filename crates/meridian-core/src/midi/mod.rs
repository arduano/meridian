pub mod ram;

use std::{fs::File, path::PathBuf, time::UNIX_EPOCH};

use enum_dispatch::enum_dispatch;

use crate::error::MeridianError;

pub const MIDI_KEY_COUNT: usize = 256;

#[derive(Debug, Clone, Copy)]
pub struct MIDIAnalysisSummary {
    pub total_blocks: u64,
    pub keys_with_notes: usize,
    pub max_blocks_per_key: usize,
    pub max_notes_in_block: usize,
    pub densest_key: usize,
    pub densest_key_notes: u64,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, Default)]
pub struct MIDIFileStats {
    pub total_notes: Option<u64>,
    pub passed_notes: Option<u64>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct MIDIViewRange {
    pub start: f64,
    pub end: f64,
}

impl MIDIViewRange {
    pub fn new(start: f64, end: f64) -> Self {
        Self { start, end }
    }

    pub fn length(&self) -> f64 {
        self.end - self.start
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MIDIFileUniqueSignature {
    pub filepath: PathBuf,
    pub length_in_bytes: u64,
    pub last_modified: u128,
}

pub(crate) fn open_file_and_signature(
    path: impl Into<PathBuf>,
) -> Result<(File, MIDIFileUniqueSignature), MeridianError> {
    let path = path.into();
    let file = File::open(&path)?;
    let metadata = file.metadata()?;
    let file_last_modified = metadata
        .modified()?
        .duration_since(UNIX_EPOCH)
        .map_err(|e| MeridianError::InvalidMidi(e.to_string()))?
        .as_micros();

    Ok((
        file,
        MIDIFileUniqueSignature {
            filepath: path,
            length_in_bytes: metadata.len(),
            last_modified: file_last_modified,
        },
    ))
}

#[derive(Debug, Clone, Copy, Default)]
pub struct MIDIColor(u32);

impl MIDIColor {
    pub fn new(r: u8, g: u8, b: u8) -> Self {
        Self((r as u32) << 16 | (g as u32) << 8 | b as u32)
    }

    pub fn new_from_hue(hue: f64) -> Self {
        let hue = hue.rem_euclid(360.0) / 60.0;
        let c = 0.78;
        let x = c * (1.0 - ((hue % 2.0) - 1.0).abs());
        let (r, g, b) = match hue as u32 {
            0 => (c, x, 0.0),
            1 => (x, c, 0.0),
            2 => (0.0, c, x),
            3 => (0.0, x, c),
            4 => (x, 0.0, c),
            _ => (c, 0.0, x),
        };
        let m = 0.12;
        Self::new(
            ((r + m) * 255.0) as u8,
            ((g + m) * 255.0) as u8,
            ((b + m) * 255.0) as u8,
        )
    }

    pub fn new_vec(tracks: usize) -> Vec<Self> {
        let count = tracks.max(1) * 16;
        let mut vec = Vec::with_capacity(count);
        for index in 0..count {
            let track = index / 16;
            let channel = index % 16;
            vec.push(Self::new_from_hue(
                ((track + channel) as f64 * -16.0) % 360.0,
            ));
        }
        vec
    }

    pub fn to_rgba(self, alpha: f32) -> [f32; 4] {
        [
            self.red() as f32 / 255.0,
            self.green() as f32 / 255.0,
            self.blue() as f32 / 255.0,
            alpha,
        ]
    }

    pub fn to_rgba_packed(self, alpha: u8) -> u32 {
        (self.red() as u32)
            | ((self.green() as u32) << 8)
            | ((self.blue() as u32) << 16)
            | ((alpha as u32) << 24)
    }

    pub fn red(&self) -> u8 {
        (self.0 >> 16) as u8
    }

    pub fn green(&self) -> u8 {
        (self.0 >> 8) as u8
    }

    pub fn blue(&self) -> u8 {
        self.0 as u8
    }
}

#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq)]
pub(crate) struct TrackAndChannel(u32);

impl TrackAndChannel {
    pub(crate) fn new(track: u32, channel: u8) -> Self {
        Self(track * 16 + channel as u32)
    }

    pub(crate) fn as_usize(self) -> usize {
        self.0 as usize
    }
}

#[derive(Debug, Clone, Copy)]
pub struct DisplacedMIDINote {
    pub start: f32,
    pub len: f32,
    pub color: MIDIColor,
}

#[allow(dead_code)]
#[enum_dispatch]
pub trait MIDIFileBase {
    fn midi_length(&self) -> Option<f64>;
    fn parsed_up_to(&self) -> Option<f64>;
    fn stats(&self) -> MIDIFileStats;
    fn allows_seeking_backward(&self) -> bool;
    fn signature(&self) -> &MIDIFileUniqueSignature;
}

pub trait MIDIFile: MIDIFileBase {
    type ColumnsViews<'a>: 'a + MIDINoteViews
    where
        Self: 'a;

    fn get_current_column_views(&mut self, time: f64, range: f64) -> Self::ColumnsViews<'_>;
}

pub trait MIDINoteViews {
    type View<'a>: 'a + MIDINoteColumnView
    where
        Self: 'a;

    fn get_column(&self, key: usize) -> Self::View<'_>;
    fn range(&self) -> MIDIViewRange;
}

pub trait MIDINoteColumnView: Send {
    type Iter<'a>: 'a + ExactSizeIterator<Item = DisplacedMIDINote> + Send
    where
        Self: 'a;

    fn iterate_displaced_notes(&self) -> Self::Iter<'_>;
}

#[enum_dispatch(MIDIFileBase)]
pub enum MIDIFileUnion {
    InRam(ram::InRamMIDIFile),
}

impl MIDIFileUnion {
    pub fn load_ram(path: impl Into<PathBuf>) -> Result<Self, MeridianError> {
        Ok(Self::InRam(ram::InRamMIDIFile::load_from_file(path)?))
    }

    pub fn get_current_column_views(&mut self, time: f64, range: f64) -> MIDIFileViewsUnion<'_> {
        match self {
            Self::InRam(file) => {
                MIDIFileViewsUnion::InRam(file.get_current_column_views(time, range))
            }
        }
    }

    pub fn analysis_summary(&self) -> MIDIAnalysisSummary {
        match self {
            Self::InRam(file) => file.analysis_summary(),
        }
    }

    pub fn key_note_counts(&self) -> [u64; MIDI_KEY_COUNT] {
        match self {
            Self::InRam(file) => file.key_note_counts(),
        }
    }
}

pub enum MIDIFileViewsUnion<'a> {
    InRam(ram::view::InRamCurrentNoteViews<'a>),
}

impl MIDIFileViewsUnion<'_> {
    pub fn get_column(&self, key: usize) -> MIDINoteColumnViewUnion<'_> {
        match self {
            Self::InRam(views) => MIDINoteColumnViewUnion::InRam(views.get_column(key)),
        }
    }

    pub fn range(&self) -> MIDIViewRange {
        match self {
            Self::InRam(views) => views.range(),
        }
    }
}

pub enum MIDINoteColumnViewUnion<'a> {
    InRam(ram::view::InRamNoteColumnView<'a>),
}

impl MIDINoteColumnViewUnion<'_> {
    pub fn iterate_displaced_notes(
        &self,
    ) -> Box<dyn ExactSizeIterator<Item = DisplacedMIDINote> + Send + '_> {
        match self {
            Self::InRam(view) => Box::new(view.iterate_displaced_notes()),
        }
    }

    pub fn for_each_displaced_note(&self, mut callback: impl FnMut(DisplacedMIDINote)) {
        match self {
            Self::InRam(view) => {
                for note in view.iterate_displaced_notes() {
                    callback(note);
                }
            }
        }
    }
}
