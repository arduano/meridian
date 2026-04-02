mod support;

use meridian_core::midi::{
    analysis::{build_buckets_from_parsed_with_progress, build_cached_midi_analysis_with_progress},
    parsed::ParsedMidiFile,
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
