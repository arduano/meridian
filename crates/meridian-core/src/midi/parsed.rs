use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::PathBuf,
    sync::OnceLock,
};

use midi_toolkit::{
    io::{DiskReader, MIDIFile as TKMIDIFile},
    sequence::event::get_channels_array_statistics,
};

use crate::error::MeridianError;

use super::{MIDIFileUniqueSignature, analysis::gzip_size_for_path, open_file_and_signature};

pub type ToolkitMidiFile = TKMIDIFile<DiskReader>;

#[derive(Debug, Clone, Copy)]
pub struct ParsedMidiHeader {
    pub format: u16,
    pub declared_track_count: u16,
    pub time_division: u16,
}

pub struct ParsedMidiFile {
    midi: ToolkitMidiFile,
    signature: MIDIFileUniqueSignature,
    header: ParsedMidiHeader,
    total_event_count: OnceLock<u64>,
    gzip_size: OnceLock<u64>,
}

impl ParsedMidiFile {
    pub fn load_from_file(path: impl Into<PathBuf>) -> Result<Self, MeridianError> {
        Self::load_from_file_with_progress(path, |_| {})
    }

    pub fn load_from_file_with_progress(
        path: impl Into<PathBuf>,
        mut progress: impl FnMut(f32),
    ) -> Result<Self, MeridianError> {
        let (mut file, signature) = open_file_and_signature(path)?;
        let header = read_header(&mut file)?;
        let declared_track_count = header.declared_track_count.max(1);
        file.seek(SeekFrom::Start(0))?;
        let mut read_progress = |tracks_done: u32| {
            progress((tracks_done as f32 / declared_track_count as f32).clamp(0.0, 1.0));
        };
        let midi = ToolkitMidiFile::open_from_stream(file, Some(&mut read_progress))
            .map_err(|e| MeridianError::MidiLoad(format!("{e:?}")))?;
        progress(1.0);

        Ok(Self {
            midi,
            signature,
            header,
            total_event_count: OnceLock::new(),
            gzip_size: OnceLock::new(),
        })
    }

    pub fn midi(&self) -> &ToolkitMidiFile {
        &self.midi
    }

    pub fn signature(&self) -> &MIDIFileUniqueSignature {
        &self.signature
    }

    pub fn header(&self) -> ParsedMidiHeader {
        self.header
    }

    pub fn cached_total_event_count(&self) -> Option<u64> {
        self.total_event_count.get().copied()
    }

    pub fn total_event_count(&self) -> Result<u64, MeridianError> {
        if let Some(total) = self.total_event_count.get() {
            return Ok(*total);
        }

        let stats = get_channels_array_statistics(self.midi.iter_all_tracks().collect())
            .map_err(|e| MeridianError::MidiLoad(format!("{e:?}")))?;
        let total = stats.total_event_count();
        let _ = self.total_event_count.set(total);
        Ok(total)
    }

    pub fn cached_gzip_size(&self) -> std::io::Result<u64> {
        if let Some(size) = self.gzip_size.get() {
            return Ok(*size);
        }

        let size = gzip_size_for_path(&self.signature.filepath)?;
        let _ = self.gzip_size.set(size);
        Ok(*self.gzip_size.get().unwrap_or(&size))
    }
}

fn read_header(file: &mut File) -> Result<ParsedMidiHeader, MeridianError> {
    let mut header = [0_u8; 14];
    file.seek(SeekFrom::Start(0))?;
    file.read_exact(&mut header)?;
    if &header[0..4] != b"MThd" {
        return Err(MeridianError::InvalidMidi(
            "missing MIDI header chunk".into(),
        ));
    }
    Ok(ParsedMidiHeader {
        format: u16::from_be_bytes([header[8], header[9]]),
        declared_track_count: u16::from_be_bytes([header[10], header[11]]),
        time_division: u16::from_be_bytes([header[12], header[13]]),
    })
}
