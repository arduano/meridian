use std::{
    fs::File,
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

use crate::error::MeridianError;

use super::MIDIFileUniqueSignature;

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

pub(crate) fn cleanup_output_file(output: &Path) {
    let _ = std::fs::remove_file(output);
}
