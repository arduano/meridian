use crate::error::MeridianError;
use crate::midi::{
    materialized::{MaterializeOptions, build_materialized_midi_with_progress},
    parsed::ParsedMidiFile,
    ram::cache::InRamMidiCache,
};

pub(super) fn build_in_ram_cache(parsed: &ParsedMidiFile) -> Result<InRamMidiCache, MeridianError> {
    build_in_ram_cache_with_progress(parsed, |_| {})
}

pub(crate) fn build_in_ram_cache_with_progress(
    parsed: &ParsedMidiFile,
    progress: impl FnMut(Option<f32>),
) -> Result<InRamMidiCache, MeridianError> {
    let materialized = build_materialized_midi_with_progress(
        parsed,
        MaterializeOptions {
            display: true,
            audio: false,
        },
        progress,
    )?;
    Ok(materialized
        .display
        .expect("display cache must exist when display materialization is requested"))
}
