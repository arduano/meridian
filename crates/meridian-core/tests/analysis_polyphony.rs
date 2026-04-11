mod support;

use meridian_core::midi::{
    analysis::{
        analyze_parsed_midi_with_progress, build_buckets_from_parsed_with_progress,
        build_cached_midi_analysis_with_progress,
    },
    parsed::ParsedMidiFile,
    MidiProcessingConfig, ProcessedMidi,
};
use midi_toolkit::events::Event;

#[test]
fn zero_velocity_note_on_does_not_inflate_polyphony_metrics() {
    let dir = support::temp_dir("meridian-core-analysis-polyphony");
    let midi = dir.join("zero-velocity-note-off.mid");
    support::write_toolkit_midi(
        &midi,
        96,
        vec![vec![
            Event::new_delta_tempo_event(0, 500_000),
            Event::new_delta_note_on_event(0, 0, 60, 100),
            Event::new_delta_note_on_event(96, 0, 60, 0),
            Event::new_delta_note_on_event(0, 0, 64, 100),
            Event::new_delta_note_off_event(96, 0, 64),
        ]],
    );

    let parsed = ParsedMidiFile::load_from_file(&midi).expect("load parsed midi");
    let cached =
        build_cached_midi_analysis_with_progress(&parsed, |_| {}).expect("build cached analysis");

    assert_eq!(cached.total_notes(), 2);
    assert_eq!(cached.notes().max_simultaneous_notes, 1);
    assert!((cached.notes().avg_simultaneous_notes - 1.0).abs() < 1e-9);

    let buckets = build_buckets_from_parsed_with_progress(&parsed, 4, cached.midi_length(), |_| {})
        .expect("build buckets");
    assert_eq!(
        buckets.iter().map(|bucket| bucket.note_starts).sum::<u64>(),
        2
    );
    assert_eq!(
        buckets
            .iter()
            .map(|bucket| bucket.active_notes)
            .max()
            .unwrap_or(0),
        1
    );
}

#[test]
fn processed_midi_polyphony_does_not_collapse_to_total_notes() {
    let dir = support::temp_dir("meridian-core-processed-polyphony");
    let midi = dir.join("processed-polyphony.mid");
    support::write_toolkit_midi(
        &midi,
        96,
        vec![vec![
            Event::new_delta_tempo_event(0, 500_000),
            Event::new_delta_note_on_event(0, 0, 60, 100),
            Event::new_delta_note_off_event(96, 0, 60),
            Event::new_delta_note_on_event(0, 0, 64, 100),
            Event::new_delta_note_off_event(96, 0, 64),
            Event::new_delta_note_on_event(0, 0, 67, 100),
            Event::new_delta_note_off_event(96, 0, 67),
        ]],
    );

    let parsed = ParsedMidiFile::load_from_file(&midi).expect("load parsed midi");
    let processed = ProcessedMidi::from_parsed(&parsed, &MidiProcessingConfig::default())
        .expect("build processed midi");
    let analysis = processed.analysis_cache();

    assert_eq!(processed.total_notes(), 3);
    assert_eq!(analysis.total_notes(), 3);
    assert_eq!(analysis.notes().max_simultaneous_notes, 1);
    assert!((analysis.notes().avg_simultaneous_notes - 1.0).abs() < 1e-9);
}

#[test]
fn zero_bucket_requests_return_empty_bucket_sets() {
    let dir = support::temp_dir("meridian-core-analysis-zero-buckets");
    let midi = dir.join("zero-buckets.mid");
    support::write_toolkit_midi(
        &midi,
        96,
        vec![vec![
            Event::new_delta_tempo_event(0, 500_000),
            Event::new_delta_note_on_event(0, 0, 60, 100),
            Event::new_delta_note_off_event(96, 0, 60),
        ]],
    );

    let parsed = ParsedMidiFile::load_from_file(&midi).expect("load parsed midi");
    let cached =
        build_cached_midi_analysis_with_progress(&parsed, |_| {}).expect("build cached analysis");

    let buckets = build_buckets_from_parsed_with_progress(&parsed, 0, cached.midi_length(), |_| {})
        .expect("build parsed buckets");
    assert!(buckets.is_empty());

    let analysis =
        analyze_parsed_midi_with_progress(&parsed, &cached, 0, |_| {}).expect("analyze midi");
    assert!(analysis.buckets.is_empty());

    let processed = ProcessedMidi::from_parsed(&parsed, &MidiProcessingConfig::default())
        .expect("build processed midi");
    let display = processed.display_cache();
    let processed_analysis = processed.analysis_cache();
    assert!(processed_analysis
        .build_buckets(&display, 0, processed_analysis.midi_length())
        .is_empty());
}
