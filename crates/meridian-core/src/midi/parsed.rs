use std::path::PathBuf;

use midi_toolkit::io::{DiskReader, MIDIFile as TKMIDIFile};

use crate::error::MeridianError;

use super::{MIDIFileUniqueSignature, open_file_and_signature};

pub type ToolkitMidiFile = TKMIDIFile<DiskReader>;

pub struct ParsedMidiFile {
    midi: ToolkitMidiFile,
    signature: MIDIFileUniqueSignature,
}

impl ParsedMidiFile {
    pub fn load_from_file(path: impl Into<PathBuf>) -> Result<Self, MeridianError> {
        let (file, signature) = open_file_and_signature(path)?;
        let midi = ToolkitMidiFile::open_from_stream(file, None)
            .map_err(|e| MeridianError::MidiLoad(format!("{e:?}")))?;
        Ok(Self { midi, signature })
    }

    pub fn midi(&self) -> &ToolkitMidiFile {
        &self.midi
    }

    pub fn signature(&self) -> &MIDIFileUniqueSignature {
        &self.signature
    }
}
