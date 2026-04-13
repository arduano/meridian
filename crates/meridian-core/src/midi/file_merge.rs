use std::path::{Path, PathBuf};

use midi_toolkit::{
    events::Event,
    io::MIDIWriter,
    sequence::event::{Delta, filter_events, merge_events_array, scale_event_ppq},
};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::MeridianError;

use super::parsed::ParsedMidiFile;

const SCAN_PROGRESS_END: f32 = 10.0;
const BODY_PROGRESS_START: f32 = 20.0;
const BODY_PROGRESS_SPAN: f32 = 80.0;
const COMPLETE_PROGRESS: f32 = 100.0;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default, TS)]
#[serde(rename_all = "snake_case")]
pub enum MidiFilesMergeMode {
    #[default]
    AppendTracks,
    MergeTracks,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default, TS)]
#[serde(default)]
pub struct MidiFilesMergeConfig {
    pub mode: MidiFilesMergeMode,
    pub normalize_metadata_track: bool,
    pub ppq_override: Option<u16>,
}

#[derive(Debug, Clone)]
pub struct MidiFilesMergeProgress {
    pub status: String,
    pub progress_percent: f32,
}

impl MidiFilesMergeProgress {
    fn new(status: impl Into<String>, progress_percent: f32) -> Self {
        Self {
            status: status.into(),
            progress_percent: progress_percent.clamp(0.0, 100.0),
        }
    }
}

#[derive(Debug, Clone)]
pub struct MidiFilesMergeSummary {
    pub output: PathBuf,
    pub input_count: usize,
    pub output_track_count: usize,
    pub output_ppq: u16,
    pub total_events: usize,
}

struct ParsedMergeInputs {
    parsed_inputs: Vec<ParsedMidiFile>,
    output_ppq: u16,
    total_tracks: usize,
    max_track_count: usize,
}

impl ParsedMergeInputs {
    fn scan(
        inputs: &[PathBuf],
        config: &MidiFilesMergeConfig,
        on_progress: &mut impl FnMut(MidiFilesMergeProgress),
    ) -> Result<Self, MeridianError> {
        on_progress(MidiFilesMergeProgress::new("Scanning merge inputs", 0.0));

        let mut parsed_inputs = Vec::with_capacity(inputs.len());
        let mut input_ppq = None;
        let mut total_tracks = 0usize;
        let mut max_track_count = 0usize;

        for (index, input) in inputs.iter().enumerate() {
            let parsed = ParsedMidiFile::load_from_file(input.clone())?;
            let parsed_ppq = merge_source_ppq(&parsed)?;
            match input_ppq {
                Some(expected_ppq)
                    if config.ppq_override.is_none() && expected_ppq != parsed_ppq =>
                {
                    return Err(MeridianError::InvalidMidi(format!(
                        "midi merge requires matching ppq unless ppq_override is set; expected {expected_ppq}, got {parsed_ppq} for {}",
                        parsed.signature().filepath.display()
                    )));
                }
                None => input_ppq = Some(parsed_ppq),
                _ => {}
            }

            total_tracks += parsed.midi().track_count();
            max_track_count = max_track_count.max(parsed.midi().track_count());
            parsed_inputs.push(parsed);

            let progress = ((index + 1) as f32 / inputs.len() as f32) * SCAN_PROGRESS_END;
            on_progress(MidiFilesMergeProgress::new(
                format!("Scanned {}", input.display()),
                progress,
            ));
        }

        Ok(Self {
            parsed_inputs,
            output_ppq: config.ppq_override.unwrap_or(input_ppq.unwrap_or(1)).max(1),
            total_tracks,
            max_track_count,
        })
    }

    fn body_track_count(&self, mode: MidiFilesMergeMode) -> usize {
        match mode {
            MidiFilesMergeMode::AppendTracks => self.total_tracks,
            MidiFilesMergeMode::MergeTracks => self.max_track_count,
        }
    }
}

struct MergeRunState {
    output: PathBuf,
    input_count: usize,
    output_ppq: u16,
    output_track_count: usize,
    total_events: usize,
}

impl MergeRunState {
    fn new(output: &Path, input_count: usize, output_ppq: u16) -> Self {
        Self {
            output: output.to_path_buf(),
            input_count,
            output_ppq,
            output_track_count: 0,
            total_events: 0,
        }
    }

    fn record_track(&mut self, total_events: usize) {
        self.output_track_count += 1;
        self.total_events += total_events;
    }

    fn finish(self) -> MidiFilesMergeSummary {
        MidiFilesMergeSummary {
            output: self.output,
            input_count: self.input_count,
            output_track_count: self.output_track_count,
            output_ppq: self.output_ppq,
            total_events: self.total_events,
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum BodyTrackFilter {
    All,
    NonMetaOnly,
}

impl BodyTrackFilter {
    fn body(normalize_metadata_track: bool) -> Self {
        if normalize_metadata_track {
            Self::NonMetaOnly
        } else {
            Self::All
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum MetadataFilter {
    MetaOnly,
    NonMetaOnly,
}

impl MetadataFilter {
    fn matches(self, event: &Event) -> bool {
        match self {
            Self::MetaOnly => is_metadata_event(event),
            Self::NonMetaOnly => !is_metadata_event(event),
        }
    }
}

pub fn merge_midi_files_to_file(
    inputs: &[PathBuf],
    output: &Path,
    config: &MidiFilesMergeConfig,
) -> Result<MidiFilesMergeSummary, MeridianError> {
    merge_midi_files_to_file_with_progress(inputs, output, config, |_| {})
}

pub fn merge_midi_files_to_file_with_progress(
    inputs: &[PathBuf],
    output: &Path,
    config: &MidiFilesMergeConfig,
    on_progress: impl FnMut(MidiFilesMergeProgress),
) -> Result<MidiFilesMergeSummary, MeridianError> {
    merge_midi_files_to_file_with_progress_cancelable(inputs, output, config, || false, on_progress)
}

pub fn merge_midi_files_to_file_with_progress_cancelable(
    inputs: &[PathBuf],
    output: &Path,
    config: &MidiFilesMergeConfig,
    is_cancelled: impl Fn() -> bool,
    mut on_progress: impl FnMut(MidiFilesMergeProgress),
) -> Result<MidiFilesMergeSummary, MeridianError> {
    if inputs.is_empty() {
        return Err(MeridianError::InvalidMidi(
            "midi merge input list is empty".into(),
        ));
    }

    let result = (|| {
        ensure_merge_not_cancelled(&is_cancelled)?;
        let parsed_inputs = ParsedMergeInputs::scan(inputs, config, &mut on_progress)?;
        let writer =
            MIDIWriter::new(output.to_string_lossy().as_ref(), parsed_inputs.output_ppq)
                .map_err(|error| MeridianError::MidiLoad(format!("midi write error: {error}")))?;
        let mut state = MergeRunState::new(output, inputs.len(), parsed_inputs.output_ppq);

        if config.normalize_metadata_track {
            ensure_merge_not_cancelled(&is_cancelled)?;
            // Emit one merged metadata-only track up front, then strip metadata from body tracks.
            on_progress(MidiFilesMergeProgress::new(
                "Normalizing metadata track",
                BODY_PROGRESS_START,
            ));
            state.record_track(write_metadata_track(&parsed_inputs, &writer)?);
        }

        write_body_tracks(
            &parsed_inputs,
            &writer,
            config,
            &mut state,
            &is_cancelled,
            &mut on_progress,
        )?;

        let mut writer = writer;
        writer
            .end()
            .map_err(|error| MeridianError::MidiLoad(format!("midi write error: {error}")))?;

        on_progress(MidiFilesMergeProgress::new(
            "Finished merging MIDI files",
            COMPLETE_PROGRESS,
        ));

        Ok(state.finish())
    })();

    if result.is_err() {
        let _ = std::fs::remove_file(output);
    }

    result
}

fn write_body_tracks(
    parsed_inputs: &ParsedMergeInputs,
    writer: &MIDIWriter,
    config: &MidiFilesMergeConfig,
    state: &mut MergeRunState,
    is_cancelled: &impl Fn() -> bool,
    on_progress: &mut impl FnMut(MidiFilesMergeProgress),
) -> Result<(), MeridianError> {
    let total_body_tracks = parsed_inputs.body_track_count(config.mode);
    let body_track_filter = BodyTrackFilter::body(config.normalize_metadata_track);

    match config.mode {
        MidiFilesMergeMode::AppendTracks => {
            for parsed in &parsed_inputs.parsed_inputs {
                for track_index in 0..parsed.midi().track_count() {
                    ensure_merge_not_cancelled(is_cancelled)?;
                    on_progress(MidiFilesMergeProgress::new(
                        format!(
                            "Copying {} track {}/{}",
                            parsed.signature().filepath.display(),
                            track_index + 1,
                            parsed.midi().track_count()
                        ),
                        merge_phase_progress(
                            state
                                .output_track_count
                                .saturating_sub(config.normalize_metadata_track as usize),
                            total_body_tracks,
                        ),
                    ));
                    state.record_track(write_single_track(
                        parsed,
                        writer,
                        track_index as u32,
                        body_track_filter,
                        parsed_inputs.output_ppq,
                    )?);
                }
            }
        }
        MidiFilesMergeMode::MergeTracks => {
            for track_index in 0..parsed_inputs.max_track_count {
                ensure_merge_not_cancelled(is_cancelled)?;
                on_progress(MidiFilesMergeProgress::new(
                    format!("Merging track {}", track_index + 1),
                    merge_phase_progress(
                        state
                            .output_track_count
                            .saturating_sub(config.normalize_metadata_track as usize),
                        total_body_tracks,
                    ),
                ));
                state.record_track(write_merged_track_index(
                    parsed_inputs,
                    writer,
                    track_index,
                    body_track_filter,
                )?);
            }
        }
    }

    Ok(())
}

fn write_metadata_track(
    parsed_inputs: &ParsedMergeInputs,
    writer: &MIDIWriter,
) -> Result<usize, MeridianError> {
    let metadata_iters = parsed_inputs
        .parsed_inputs
        .iter()
        .flat_map(|parsed| {
            (0..parsed.midi().track_count()).filter_map(move |track_index| {
                meta_track_events(parsed, track_index as u32, parsed_inputs.output_ppq)
            })
        })
        .collect::<Vec<_>>();
    write_track_from_iterator(writer, merge_events_array(metadata_iters))
}

fn write_merged_track_index(
    parsed_inputs: &ParsedMergeInputs,
    writer: &MIDIWriter,
    track_index: usize,
    filter: BodyTrackFilter,
) -> Result<usize, MeridianError> {
    // Keep the branch above the iterator boundary so each path stays monomorphic.
    match filter {
        BodyTrackFilter::All => {
            let track_iters = parsed_inputs
                .parsed_inputs
                .iter()
                .filter_map(|parsed| {
                    all_track_events(parsed, track_index as u32, parsed_inputs.output_ppq)
                })
                .collect::<Vec<_>>();
            write_track_from_iterator(writer, merge_events_array(track_iters))
        }
        BodyTrackFilter::NonMetaOnly => {
            let track_iters = parsed_inputs
                .parsed_inputs
                .iter()
                .filter_map(|parsed| {
                    non_meta_track_events(parsed, track_index as u32, parsed_inputs.output_ppq)
                })
                .collect::<Vec<_>>();
            write_track_from_iterator(writer, merge_events_array(track_iters))
        }
    }
}

fn write_single_track(
    parsed: &ParsedMidiFile,
    writer: &MIDIWriter,
    track_index: u32,
    filter: BodyTrackFilter,
    output_ppq: u16,
) -> Result<usize, MeridianError> {
    // Same idea as merged-track writing: branch once, then stream a concrete pipeline.
    match filter {
        BodyTrackFilter::All => {
            let iter = all_track_events(parsed, track_index, output_ppq)
                .expect("track iteration should exist for a known track index");
            write_track_from_iterator(writer, iter)
        }
        BodyTrackFilter::NonMetaOnly => {
            let iter = non_meta_track_events(parsed, track_index, output_ppq)
                .expect("track iteration should exist for a known track index");
            write_track_from_iterator(writer, iter)
        }
    }
}

fn write_track_from_iterator(
    writer: &MIDIWriter,
    iter: impl Iterator<Item = Result<Delta<u64, Event>, MeridianError>>,
) -> Result<usize, MeridianError> {
    let mut track_writer = writer
        .try_open_next_track()
        .map_err(|error| MeridianError::MidiLoad(format!("midi write error: {error}")))?;
    let mut total_events = 0usize;

    for event in iter {
        let event = event?;
        track_writer
            .write_event(event)
            .map_err(|error| MeridianError::MidiLoad(format!("midi write error: {error}")))?;
        total_events += 1;
    }

    track_writer
        .end()
        .map_err(|error| MeridianError::MidiLoad(format!("midi write error: {error}")))?;

    Ok(total_events)
}

fn merge_source_ppq(parsed: &ParsedMidiFile) -> Result<u16, MeridianError> {
    if (parsed.header().time_division & 0x8000) != 0 {
        return Err(MeridianError::InvalidMidi(
            "timecode MIDI files are not supported yet".into(),
        ));
    }

    Ok(parsed.midi().ppq().max(1))
}

fn all_track_events(
    parsed: &ParsedMidiFile,
    track_index: u32,
    output_ppq: u16,
) -> Option<impl Iterator<Item = Result<Delta<u64, Event>, MeridianError>> + '_> {
    let source_ppq = merge_source_ppq(parsed).ok()?;
    let track = parsed.midi().iter_track(track_index)?;
    let events = track.map(map_track_event_result);

    // Always wrap with PPQ scaling so the downstream iterator shape stays uniform.
    Some(scale_event_ppq(
        events,
        source_ppq as u64,
        output_ppq.max(1) as u64,
    ))
}

fn meta_track_events(
    parsed: &ParsedMidiFile,
    track_index: u32,
    output_ppq: u16,
) -> Option<impl Iterator<Item = Result<Delta<u64, Event>, MeridianError>> + '_> {
    filtered_track_events(parsed, track_index, output_ppq, MetadataFilter::MetaOnly)
}

fn non_meta_track_events(
    parsed: &ParsedMidiFile,
    track_index: u32,
    output_ppq: u16,
) -> Option<impl Iterator<Item = Result<Delta<u64, Event>, MeridianError>> + '_> {
    filtered_track_events(parsed, track_index, output_ppq, MetadataFilter::NonMetaOnly)
}

fn filtered_track_events(
    parsed: &ParsedMidiFile,
    track_index: u32,
    output_ppq: u16,
    metadata_filter: MetadataFilter,
) -> Option<impl Iterator<Item = Result<Delta<u64, Event>, MeridianError>> + '_> {
    let source_ppq = merge_source_ppq(parsed).ok()?;
    let track = parsed.midi().iter_track(track_index)?;
    let events = track.map(map_track_event_result);
    // `filter_events` preserves delta timing across removed events; `filter_map` would not.
    let filtered = filter_events(events, move |event: &Delta<u64, Event>| {
        metadata_filter.matches(&event.event)
    });

    Some(scale_event_ppq(
        filtered,
        source_ppq as u64,
        output_ppq.max(1) as u64,
    ))
}

fn is_metadata_event(event: &Event) -> bool {
    matches!(
        event,
        Event::Text(_)
            | Event::UnknownMeta(_)
            | Event::Color(_)
            | Event::ChannelPrefix(_)
            | Event::MIDIPort(_)
            | Event::Tempo(_)
            | Event::SMPTEOffset(_)
            | Event::TimeSignature(_)
            | Event::KeySignature(_)
    )
}

fn merge_phase_progress(completed_units: usize, total_units: usize) -> f32 {
    BODY_PROGRESS_START + (completed_units as f32 / total_units.max(1) as f32) * BODY_PROGRESS_SPAN
}

fn ensure_merge_not_cancelled(is_cancelled: &impl Fn() -> bool) -> Result<(), MeridianError> {
    if is_cancelled() {
        Err(MeridianError::Cancelled("midi merge cancelled".into()))
    } else {
        Ok(())
    }
}

fn map_track_event_result(
    event: Result<Delta<u64, Event>, impl std::fmt::Debug>,
) -> Result<Delta<u64, Event>, MeridianError> {
    event.map_err(|error| MeridianError::MidiLoad(format!("{error:?}")))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::{
        MidiFilesMergeConfig, MidiFilesMergeMode, merge_midi_files_to_file,
        merge_midi_files_to_file_with_progress,
    };
    use crate::error::MeridianError;
    use crate::midi::parsed::ParsedMidiFile;
    use crate::midi::test_support::{
        TestDir, key_signature, note_off, note_on, read_track_events, tempo, text, time_signature,
        write_toolkit_midi,
    };

    fn write_raw_midi(path: &std::path::Path, bytes: &[u8]) {
        fs::write(path, bytes).expect("write raw midi bytes");
    }

    fn write_timecode_midi(path: &std::path::Path) {
        write_raw_midi(
            path,
            &[
                0x4d, 0x54, 0x68, 0x64, 0x00, 0x00, 0x00, 0x06, 0x00, 0x00, 0x00, 0x01, 0xe7, 0x28,
                0x4d, 0x54, 0x72, 0x6b, 0x00, 0x00, 0x00, 0x04, 0x00, 0xff, 0x2f, 0x00,
            ],
        );
    }

    fn write_invalid_track_midi(path: &std::path::Path) {
        write_raw_midi(
            path,
            &[
                0x4d, 0x54, 0x68, 0x64, 0x00, 0x00, 0x00, 0x06, 0x00, 0x00, 0x00, 0x01, 0x00, 0x60,
                0x4d, 0x54, 0x72, 0x6b, 0x00, 0x00, 0x00, 0x01, 0x00,
            ],
        );
    }

    #[test]
    fn append_tracks_concatenates_source_tracks() {
        let dir = TestDir::new("append_tracks");
        let input_a = dir.path("input_a.mid");
        let input_b = dir.path("input_b.mid");
        let output = dir.path("output.mid");

        write_toolkit_midi(
            &input_a,
            480,
            &[
                vec![note_on(10, 0, 60, 100), note_off(20, 0, 60)],
                vec![note_on(30, 1, 61, 90)],
            ],
        );
        write_toolkit_midi(&input_b, 480, &[vec![note_on(40, 2, 62, 80)]]);

        let summary = merge_midi_files_to_file(
            &[input_a.clone(), input_b.clone()],
            &output,
            &MidiFilesMergeConfig::default(),
        )
        .expect("append merge should succeed");

        let merged = ParsedMidiFile::load_from_file(output).expect("merged midi should parse");
        assert_eq!(summary.output_track_count, 3);
        assert_eq!(merged.midi().track_count(), 3);
        assert_eq!(
            read_track_events(&merged, 0),
            vec![note_on(10, 0, 60, 100), note_off(20, 0, 60)]
        );
        assert_eq!(read_track_events(&merged, 1), vec![note_on(30, 1, 61, 90)]);
        assert_eq!(read_track_events(&merged, 2), vec![note_on(40, 2, 62, 80)]);
    }

    #[test]
    fn merge_tracks_merges_tracks_by_index() {
        let dir = TestDir::new("merge_tracks");
        let input_a = dir.path("input_a.mid");
        let input_b = dir.path("input_b.mid");
        let output = dir.path("output.mid");

        write_toolkit_midi(
            &input_a,
            480,
            &[vec![note_on(10, 0, 60, 100)], vec![note_on(30, 1, 62, 90)]],
        );
        write_toolkit_midi(
            &input_b,
            480,
            &[vec![note_on(5, 2, 64, 80)], vec![note_on(40, 3, 65, 70)]],
        );

        let summary = merge_midi_files_to_file(
            &[input_a.clone(), input_b.clone()],
            &output,
            &MidiFilesMergeConfig {
                mode: MidiFilesMergeMode::MergeTracks,
                ..Default::default()
            },
        )
        .expect("merge-by-index should succeed");

        let merged = ParsedMidiFile::load_from_file(output).expect("merged midi should parse");
        assert_eq!(summary.output_track_count, 2);
        assert_eq!(merged.midi().track_count(), 2);
        assert_eq!(
            read_track_events(&merged, 0),
            vec![note_on(5, 2, 64, 80), note_on(5, 0, 60, 100)]
        );
        assert_eq!(
            read_track_events(&merged, 1),
            vec![note_on(30, 1, 62, 90), note_on(10, 3, 65, 70)]
        );
    }

    #[test]
    fn normalize_metadata_track_moves_metadata_and_preserves_body_deltas() {
        let dir = TestDir::new("normalize_metadata");
        let input_a = dir.path("input_a.mid");
        let input_b = dir.path("input_b.mid");
        let output = dir.path("output.mid");

        write_toolkit_midi(
            &input_a,
            480,
            &[vec![
                text(0, "A"),
                note_on(10, 0, 60, 100),
                tempo(15, 500_000),
                note_off(20, 0, 60),
            ]],
        );
        write_toolkit_midi(
            &input_b,
            480,
            &[vec![
                time_signature(7, 4, 2, 24, 8),
                note_on(11, 1, 62, 90),
                key_signature(13, 1, 0),
                note_off(17, 1, 62),
            ]],
        );

        merge_midi_files_to_file(
            &[input_a.clone(), input_b.clone()],
            &output,
            &MidiFilesMergeConfig {
                normalize_metadata_track: true,
                ..Default::default()
            },
        )
        .expect("metadata normalization should succeed");

        let merged = ParsedMidiFile::load_from_file(output).expect("merged midi should parse");
        assert_eq!(merged.midi().track_count(), 3);
        assert_eq!(
            read_track_events(&merged, 0),
            vec![
                text(0, "A"),
                time_signature(7, 4, 2, 24, 8),
                tempo(18, 500_000),
                key_signature(6, 1, 0)
            ]
        );
        assert_eq!(
            read_track_events(&merged, 1),
            vec![note_on(10, 0, 60, 100), note_off(35, 0, 60)]
        );
        assert_eq!(
            read_track_events(&merged, 2),
            vec![note_on(18, 1, 62, 90), note_off(30, 1, 62)]
        );
    }

    #[test]
    fn ppq_override_rescales_each_source_track() {
        let dir = TestDir::new("ppq_override");
        let input_a = dir.path("input_a.mid");
        let input_b = dir.path("input_b.mid");
        let output = dir.path("output.mid");

        write_toolkit_midi(&input_a, 480, &[vec![note_on(10, 0, 60, 100)]]);
        write_toolkit_midi(&input_b, 960, &[vec![note_on(20, 1, 62, 90)]]);

        let summary = merge_midi_files_to_file(
            &[input_a.clone(), input_b.clone()],
            &output,
            &MidiFilesMergeConfig {
                ppq_override: Some(240),
                ..Default::default()
            },
        )
        .expect("ppq override merge should succeed");

        let merged = ParsedMidiFile::load_from_file(output).expect("merged midi should parse");
        assert_eq!(summary.output_ppq, 240);
        assert_eq!(merged.midi().ppq(), 240);
        assert_eq!(read_track_events(&merged, 0), vec![note_on(5, 0, 60, 100)]);
        assert_eq!(read_track_events(&merged, 1), vec![note_on(5, 1, 62, 90)]);
    }

    #[test]
    fn merge_requires_matching_ppq_without_override() {
        let dir = TestDir::new("ppq_mismatch");
        let input_a = dir.path("input_a.mid");
        let input_b = dir.path("input_b.mid");
        let output = dir.path("output.mid");

        write_toolkit_midi(&input_a, 480, &[vec![note_on(10, 0, 60, 100)]]);
        write_toolkit_midi(&input_b, 960, &[vec![note_on(10, 1, 62, 90)]]);

        let error = merge_midi_files_to_file(
            &[input_a, input_b],
            &output,
            &MidiFilesMergeConfig::default(),
        )
        .expect_err("mismatched ppq should fail without override");

        assert!(matches!(error, MeridianError::InvalidMidi(_)));
    }

    #[test]
    fn merge_rejects_timecode_divisions() {
        let dir = TestDir::new("timecode_division");
        let input = dir.path("input.mid");
        let output = dir.path("output.mid");

        write_timecode_midi(&input);

        let error = merge_midi_files_to_file(&[input], &output, &MidiFilesMergeConfig::default())
            .expect_err("timecode divisions should be rejected");

        assert!(
            matches!(error, MeridianError::InvalidMidi(message) if message.contains("timecode MIDI files are not supported yet"))
        );
        assert!(!output.exists());
    }

    #[test]
    fn merge_cleans_up_output_on_non_cancel_error() {
        let dir = TestDir::new("cleanup_on_error");
        let input = dir.path("broken.mid");
        let output = dir.path("output.mid");

        write_invalid_track_midi(&input);

        let error = merge_midi_files_to_file(&[input], &output, &MidiFilesMergeConfig::default())
            .expect_err("invalid track data should fail during merge");

        assert!(matches!(error, MeridianError::MidiLoad(_)));
        assert!(!output.exists(), "failed merge output should be removed");
    }

    #[test]
    fn progress_reports_finish() {
        let dir = TestDir::new("progress");
        let input = dir.path("input.mid");
        let output = dir.path("output.mid");
        let mut progress_updates = Vec::new();

        write_toolkit_midi(&input, 480, &[vec![note_on(10, 0, 60, 100)]]);

        merge_midi_files_to_file_with_progress(
            &[input],
            &output,
            &MidiFilesMergeConfig::default(),
            |progress| progress_updates.push((progress.status, progress.progress_percent)),
        )
        .expect("merge should succeed");

        assert!(!progress_updates.is_empty());
        let (last_status, last_progress) = progress_updates
            .last()
            .expect("progress should have a final update");
        assert_eq!(last_status, "Finished merging MIDI files");
        assert_eq!(*last_progress, 100.0);
    }
}
