use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use midi_toolkit::{
    events::{Event, TextEventKind},
    io::MIDIWriter,
    sequence::event::Delta,
};

use super::parsed::ParsedMidiFile;

static NEXT_TEST_ID: AtomicU64 = AtomicU64::new(1);

pub(crate) struct TestDir {
    path: PathBuf,
}

impl TestDir {
    pub(crate) fn new(label: &str) -> Self {
        let id = NEXT_TEST_ID.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "meridian-midi-tests-{label}-{}-{id}",
            std::process::id()
        ));
        fs::create_dir_all(&path).expect("create test directory");
        Self { path }
    }

    pub(crate) fn path(&self, name: &str) -> PathBuf {
        self.path.join(name)
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

pub(crate) fn write_toolkit_midi(path: &Path, ppq: u16, tracks: &[Vec<Delta<u64, Event>>]) {
    let writer = MIDIWriter::new(path.to_string_lossy().as_ref(), ppq).expect("test midi writer");
    for track in tracks {
        let mut track_writer = writer.try_open_next_track().expect("open test track");
        for event in track {
            track_writer
                .write_event(event.clone())
                .expect("write test event");
        }
        track_writer.end().expect("end test track");
    }
    let mut writer = writer;
    writer.end().expect("end test midi");
}

pub(crate) fn read_track_events(
    parsed: &ParsedMidiFile,
    track_index: u32,
) -> Vec<Delta<u64, Event>> {
    parsed
        .midi()
        .iter_track(track_index)
        .expect("track should exist")
        .map(|event| event.expect("track event should parse"))
        .filter(|event| !matches!(event.event, Event::TrackStart(_)))
        .collect()
}

pub(crate) fn note_on(delta: u64, channel: u8, key: u8, velocity: u8) -> Delta<u64, Event> {
    Event::new_delta_note_on_event(delta, channel, key, velocity)
}

pub(crate) fn note_off(delta: u64, channel: u8, key: u8) -> Delta<u64, Event> {
    Event::new_delta_note_off_event(delta, channel, key)
}

pub(crate) fn text(delta: u64, value: &str) -> Delta<u64, Event> {
    Event::new_delta_text_event(delta, TextEventKind::TrackName, value.as_bytes().to_vec())
}

pub(crate) fn tempo(delta: u64, tempo: u32) -> Delta<u64, Event> {
    Event::new_delta_tempo_event(delta, tempo)
}

pub(crate) fn time_signature(
    delta: u64,
    numerator: u8,
    denominator: u8,
    ticks_per_click: u8,
    bb: u8,
) -> Delta<u64, Event> {
    Event::new_delta_time_signature_event(delta, numerator, denominator, ticks_per_click, bb)
}

pub(crate) fn key_signature(delta: u64, sf: u8, mi: u8) -> Delta<u64, Event> {
    Event::new_delta_key_signature_event(delta, sf, mi)
}
