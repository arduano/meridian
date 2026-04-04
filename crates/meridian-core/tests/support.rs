#![allow(dead_code)]

use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use midi_toolkit::{events::Event, io::MIDIWriter, sequence::event::Delta};

pub fn temp_dir(prefix: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "{prefix}-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    fs::create_dir_all(&dir).expect("create temp test dir");
    dir
}

fn repo_midis_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets/midis")
}

pub fn fixture_or_temp(name: &str, bytes: &[u8]) -> PathBuf {
    for candidate in [
        repo_midis_dir().join(name),
        repo_midis_dir().join("test").join(name),
        repo_midis_dir().join("piano").join(name),
    ] {
        if candidate.exists() {
            return candidate;
        }
    }

    let dir = temp_dir("meridian-core-midi-fixture");
    let path = dir.join(name);
    fs::write(&path, bytes).expect("write midi fixture");
    path
}

pub fn write_test_midi() -> PathBuf {
    fixture_or_temp(
        "smoke-two-notes.mid",
        &[
            0x4d, 0x54, 0x68, 0x64, 0x00, 0x00, 0x00, 0x06, 0x00, 0x00, 0x00, 0x01, 0x00, 0x60,
            0x4d, 0x54, 0x72, 0x6b, 0x00, 0x00, 0x00, 0x1b, 0x00, 0xff, 0x51, 0x03, 0x07, 0xa1,
            0x20, 0x00, 0x90, 0x3c, 0x64, 0x30, 0x90, 0x40, 0x64, 0x30, 0x80, 0x3c, 0x40, 0x30,
            0x80, 0x40, 0x40, 0x00, 0xff, 0x2f, 0x00,
        ],
    )
}

fn encode_vlq(mut value: u32) -> Vec<u8> {
    let mut bytes = vec![(value & 0x7f) as u8];
    value >>= 7;
    while value > 0 {
        bytes.push(((value & 0x7f) as u8) | 0x80);
        value >>= 7;
    }
    bytes.reverse();
    bytes
}

pub fn write_tempo_staircase_midi() -> PathBuf {
    #[derive(Clone, Copy)]
    enum EventKind {
        Tempo(u32),
        NoteOn(u8),
        NoteOff(u8),
        EndOfTrack,
    }

    let dir = temp_dir("meridian-core-staircase-test");
    let path = dir.join("tempo_staircase.mid");

    let mut events = vec![
        (0_u32, 0_u8, EventKind::Tempo(379_747)),
        (13_438, 0, EventKind::Tempo(400_000)),
        (14_210, 0, EventKind::Tempo(387_097)),
        (17_284, 0, EventKind::Tempo(379_747)),
        (28_802, 0, EventKind::Tempo(400_000)),
        (29_568, 0, EventKind::Tempo(392_157)),
        (32_640, 0, EventKind::Tempo(384_615)),
        (35_716, 0, EventKind::Tempo(379_747)),
        (50_304, 0, EventKind::Tempo(382_166)),
        (51_074, 0, EventKind::Tempo(387_097)),
        (54_148, 0, EventKind::Tempo(379_747)),
        (56_448, 0, EventKind::Tempo(394_737)),
        (57_216, 0, EventKind::Tempo(379_747)),
        (65_676, 0, EventKind::Tempo(382_166)),
        (66_432, 0, EventKind::Tempo(392_157)),
        (69_504_u32, 0_u8, EventKind::Tempo(1_276_596)),
        (69_888, 0, EventKind::Tempo(714_286)),
        (69_912, 0, EventKind::Tempo(631_579)),
        (69_936, 0, EventKind::Tempo(612_245)),
        (69_960, 0, EventKind::Tempo(571_429)),
        (70_008, 0, EventKind::Tempo(606_061)),
        (70_032, 0, EventKind::Tempo(645_161)),
        (70_056, 0, EventKind::Tempo(705_882)),
        (70_080, 0, EventKind::Tempo(759_494)),
        (70_104, 0, EventKind::Tempo(779_221)),
        (70_128, 0, EventKind::Tempo(833_333)),
        (70_152, 0, EventKind::Tempo(857_143)),
        (70_176, 0, EventKind::Tempo(937_500)),
        (70_200, 0, EventKind::Tempo(1_034_483)),
        (70_224, 0, EventKind::Tempo(1_071_429)),
        (70_248, 0, EventKind::Tempo(1_714_286)),
        (70_272, 0, EventKind::Tempo(500_000)),
    ];

    let staircase_notes = [
        (70_176_u32, 60_u8),
        (70_200, 61),
        (70_224, 62),
        (70_248, 63),
        (70_272, 64),
    ];
    for (tick, key) in staircase_notes {
        events.push((tick, 1, EventKind::NoteOn(key)));
        events.push((tick + 48, 2, EventKind::NoteOff(key)));
    }
    events.push((70_400, 3, EventKind::EndOfTrack));
    events.sort_by_key(|(tick, order, _)| (*tick, *order));

    let mut track = Vec::new();
    let mut previous_tick = 0_u32;
    for (tick, _, event) in events {
        track.extend_from_slice(&encode_vlq(tick.saturating_sub(previous_tick)));
        previous_tick = tick;
        match event {
            EventKind::Tempo(mpq) => {
                track.extend_from_slice(&[0xff, 0x51, 0x03]);
                track.push(((mpq >> 16) & 0xff) as u8);
                track.push(((mpq >> 8) & 0xff) as u8);
                track.push((mpq & 0xff) as u8);
            }
            EventKind::NoteOn(key) => track.extend_from_slice(&[0x90, key, 0x64]),
            EventKind::NoteOff(key) => track.extend_from_slice(&[0x80, key, 0x40]),
            EventKind::EndOfTrack => track.extend_from_slice(&[0xff, 0x2f, 0x00]),
        }
    }

    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"MThd");
    bytes.extend_from_slice(&6_u32.to_be_bytes());
    bytes.extend_from_slice(&0_u16.to_be_bytes());
    bytes.extend_from_slice(&1_u16.to_be_bytes());
    bytes.extend_from_slice(&96_u16.to_be_bytes());
    bytes.extend_from_slice(b"MTrk");
    bytes.extend_from_slice(&(track.len() as u32).to_be_bytes());
    bytes.extend_from_slice(&track);
    fs::write(&path, bytes).expect("write staircase midi fixture");
    path
}

pub fn write_toolkit_midi(path: &Path, ppq: u16, tracks: Vec<Vec<Delta<u64, Event>>>) {
    let writer = MIDIWriter::new(path.to_string_lossy().as_ref(), ppq).expect("create midi writer");
    for track in tracks {
        let mut track_writer = writer.try_open_next_track().expect("open midi track");
        track_writer
            .write_events_iter(track.into_iter())
            .expect("write midi track");
        track_writer.end().expect("finish midi track");
    }
    let mut writer = writer;
    writer.end().expect("finish midi writer");
}

pub fn ffmpeg_available() -> bool {
    std::process::Command::new("ffmpeg")
        .arg("-version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}
