use std::{
    fs,
    path::PathBuf,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use meridian_core::{
    PROTOCOL_VERSION,
    midi::{
        EventFilterConfig, FileTimeProcessingConfig, MergeProcessingConfig,
        MidiFileProcessingConfig, MidiFileSelection, MidiMergeMode, StructureProcessingConfig,
        TrimProcessingConfig, analysis::MidiAnalysisKind,
    },
    protocol::{
        CoreCommand, CoreEvent, JsonRequest, JsonResponse, MidiProcessEvent, MidiProcessStatus,
    },
    render::{DisplayTimeSpace, SceneLayout},
    spawn_core,
};
use midi_toolkit::{
    events::Event,
    io::{MIDIFile as ToolkitMidiFile, MIDIWriter},
    sequence::event::Delta,
};

fn write_test_midi() -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "meridian-core-test-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    fs::create_dir_all(&dir).expect("create temp test dir");
    let path = dir.join("fixture.mid");
    let bytes = [
        0x4d, 0x54, 0x68, 0x64, 0x00, 0x00, 0x00, 0x06, 0x00, 0x00, 0x00, 0x01, 0x00, 0x60, 0x4d,
        0x54, 0x72, 0x6b, 0x00, 0x00, 0x00, 0x1b, 0x00, 0xff, 0x51, 0x03, 0x07, 0xa1, 0x20, 0x00,
        0x90, 0x3c, 0x64, 0x30, 0x90, 0x40, 0x64, 0x30, 0x80, 0x3c, 0x40, 0x30, 0x80, 0x40, 0x40,
        0x00, 0xff, 0x2f, 0x00,
    ];
    fs::write(&path, bytes).expect("write midi fixture");
    path
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

fn write_tempo_staircase_midi() -> PathBuf {
    #[derive(Clone, Copy)]
    enum EventKind {
        Tempo(u32),
        NoteOn(u8),
        NoteOff(u8),
        EndOfTrack,
    }

    let dir = std::env::temp_dir().join(format!(
        "meridian-core-staircase-test-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    fs::create_dir_all(&dir).expect("create temp test dir");
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

fn write_toolkit_midi(path: &PathBuf, ppq: u16, tracks: Vec<Vec<Delta<u64, Event>>>) {
    let writer = MIDIWriter::new(path.to_string_lossy().as_ref(), ppq).expect("create midi writer");
    for track in tracks {
        let mut track_writer = writer.open_next_track();
        track_writer
            .write_events_iter(track.into_iter())
            .expect("write midi track");
        track_writer.end().expect("finish midi track");
    }
    let mut writer = writer;
    writer.end().expect("finish midi writer");
}

#[test]
fn stateful_core_projects_a_frame() {
    let midi = write_test_midi();
    let core = spawn_core();

    core.request(CoreCommand::LoadMidi { path: midi })
        .expect("load midi");
    core.request(CoreCommand::SetTime { time: 0.25 })
        .expect("set time");
    core.request(CoreCommand::SetViewport {
        width: 320,
        height: 180,
    })
    .expect("set viewport");

    let frame = core
        .render_frame(Some(320), Some(180))
        .expect("render frame");

    assert_eq!(frame.state.total_notes, 2);
    assert_eq!(frame.stats.visible_notes, 2);
    assert_eq!(frame.stats.active_keys, 2);
    assert!(frame.stats.total_quads > 0);
    assert!(frame.scene.total_quads() > 0);
}

#[test]
fn save_frame_supports_png_and_rgba() {
    let midi = write_test_midi();
    let out_dir = midi.parent().unwrap().to_path_buf();
    let png_path = out_dir.join("frame.png");
    let rgba_path = out_dir.join("frame.rgba");
    let core = spawn_core();

    core.request(CoreCommand::LoadMidi { path: midi })
        .expect("load midi");
    core.request(CoreCommand::SetTime { time: 0.25 })
        .expect("set time");
    core.request(CoreCommand::SetViewport {
        width: 320,
        height: 180,
    })
    .expect("set viewport");

    let png_events = core
        .request(CoreCommand::SaveFrame {
            output: png_path.clone(),
            format: None,
            viewport_width: Some(320),
            viewport_height: Some(180),
        })
        .expect("save png");
    let rgba_events = core
        .request(CoreCommand::SaveFrame {
            output: rgba_path.clone(),
            format: None,
            viewport_width: Some(320),
            viewport_height: Some(180),
        })
        .expect("save rgba");

    let png_bytes = fs::read(&png_path).expect("read png");
    let rgba_bytes = fs::read(&rgba_path).expect("read rgba");

    assert!(matches!(
        png_events.as_slice(),
        [CoreEvent::FrameSaved { bytes_written, .. }] if *bytes_written > 0
    ));
    assert!(matches!(
        rgba_events.as_slice(),
        [CoreEvent::FrameSaved { bytes_written, .. }] if *bytes_written > 0
    ));
    assert_eq!(&png_bytes[..8], b"\x89PNG\r\n\x1a\n");
    assert_eq!(rgba_bytes.len(), 320 * 180 * 4);
}

#[test]
fn json_protocol_defaults_version_and_render_response_is_lightweight() {
    let midi = write_test_midi();
    let core = spawn_core();
    core.request(CoreCommand::LoadMidi { path: midi })
        .expect("load midi");
    core.request(CoreCommand::SetViewport {
        width: 320,
        height: 180,
    })
    .expect("set viewport");

    let request: JsonRequest =
        serde_json::from_str(r#"{"id":7,"command":{"type":"get_state"}}"#).expect("json request");
    assert_eq!(request.protocol_version, PROTOCOL_VERSION);

    let events = core
        .request(CoreCommand::RenderFrame {
            viewport_width: Some(320),
            viewport_height: Some(180),
        })
        .expect("render stats");
    let response = JsonResponse {
        protocol_version: PROTOCOL_VERSION,
        id: Some(7),
        events,
    };
    let json = serde_json::to_string(&response).expect("serialize response");

    assert!(json.contains("\"frame_projected\""));
    assert!(!json.contains("\"positions\""));
    assert!(!json.contains("\"note_layers\""));
}

#[test]
fn midi_analysis_job_runs_without_building_display_cache() {
    let midi = write_test_midi();
    let core = spawn_core();

    let parsed_events = core
        .request(CoreCommand::LoadParsedMidi { path: midi })
        .expect("load parsed midi");
    let parsed_midi_id = parsed_events
        .iter()
        .find_map(|event| match event {
            CoreEvent::ParsedMidiLoaded { parsed_midi_id, .. } => Some(*parsed_midi_id),
            _ => None,
        })
        .expect("parsed midi id");

    let status_events = core
        .request(CoreCommand::StartMidiAnalysisJob {
            parsed_midi_id,
            display_cache_id: None,
            kinds: vec![
                MidiAnalysisKind::File,
                MidiAnalysisKind::Summary,
                MidiAnalysisKind::Events,
                MidiAnalysisKind::Notes,
                MidiAnalysisKind::Tempo,
                MidiAnalysisKind::Buckets,
            ],
            bucket_count: Some(4),
        })
        .expect("start midi analysis job");

    let job_id = match status_events.as_slice() {
        [CoreEvent::MidiAnalysisJobStatus { status }] => match status {
            meridian_core::protocol::MidiAnalysisJobStatus::Running { job_id, .. } => *job_id,
            other => panic!("unexpected initial status: {other:?}"),
        },
        other => panic!("unexpected status events: {other:?}"),
    };

    for _ in 0..20 {
        let events = core
            .request(CoreCommand::GetMidiAnalysisJobStatus { job_id })
            .expect("analysis job status");
        match events.as_slice() {
            [CoreEvent::MidiAnalysisJobStatus { status }] => match status {
                meridian_core::protocol::MidiAnalysisJobStatus::Finished { result, .. } => {
                    assert_eq!(result.total_notes, 2);
                    assert_eq!(result.buckets.len(), 4);
                    assert_eq!(
                        result
                            .buckets
                            .iter()
                            .map(|bucket| bucket.note_starts)
                            .sum::<u64>(),
                        2
                    );
                    assert!(result.buckets.iter().any(|bucket| bucket.active_notes > 0));
                    assert_eq!(result.events.note_on_events, 2);
                    assert_eq!(result.events.note_off_events, 2);
                    assert_eq!(result.file.declared_track_count, 1);
                    return;
                }
                meridian_core::protocol::MidiAnalysisJobStatus::Failed { message, .. } => {
                    panic!("analysis job failed: {message}")
                }
                meridian_core::protocol::MidiAnalysisJobStatus::Running { .. } => {
                    std::thread::sleep(Duration::from_millis(10));
                }
            },
            other => panic!("unexpected analysis job status response: {other:?}"),
        }
    }
    panic!("analysis job did not finish");
}

#[test]
fn process_midi_files_merges_trims_and_writes_output() {
    let dir = std::env::temp_dir().join(format!(
        "meridian-core-process-test-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    fs::create_dir_all(&dir).expect("create temp test dir");
    let midi_a = dir.join("a.mid");
    let midi_b = dir.join("b.mid");
    let output = dir.join("out.mid");

    write_toolkit_midi(
        &midi_a,
        96,
        vec![vec![
            Event::new_delta_tempo_event(0, 600_000),
            Event::new_delta_program_change_event(0, 0, 5),
            Event::new_delta_note_on_event(48, 0, 60, 100),
            Event::new_delta_note_off_event(96, 0, 60),
        ]],
    );
    write_toolkit_midi(
        &midi_b,
        48,
        vec![vec![
            Event::new_delta_note_on_event(0, 1, 65, 90),
            Event::new_delta_note_off_event(48, 1, 65),
        ]],
    );

    let core = spawn_core();
    let mut config = MidiFileProcessingConfig::default();
    config.time = FileTimeProcessingConfig {
        offset_ticks: 0,
        ppq_override: Some(120),
        tempo_override: Some(500_000),
        trim: Some(TrimProcessingConfig {
            start_tick: 24,
            end_tick: Some(120),
            inject_edge_state: true,
            close_open_notes_at_end: true,
        }),
    };
    config.notes.transpose = 12;
    config.events = EventFilterConfig::default();
    config.structure = StructureProcessingConfig {
        split_channels: false,
        collapse_tracks: true,
        remove_empty_tracks: true,
        drop_orphan_note_offs: true,
    };
    config.merge = MergeProcessingConfig {
        mode: MidiMergeMode::FlattenToSingleTrack,
    };

    let events = core
        .request(CoreCommand::ProcessMidiFiles {
            selection: MidiFileSelection {
                inputs: vec![midi_a.clone(), midi_b.clone()],
            },
            output: output.clone(),
            config,
        })
        .expect("process midi files");

    assert!(matches!(
        events.as_slice(),
        [CoreEvent::MidiFilesProcessed {
            output: processed_output,
            output_track_count: 1,
            output_ppq: 120,
            ..
        }] if processed_output == &output
    ));

    let written = ToolkitMidiFile::open_in_ram(&output, None).expect("open output midi");
    assert_eq!(written.ppq(), 120);
    assert_eq!(written.track_count(), 1);

    let mut seen_tempo = 0;
    let mut seen_program = 0;
    let mut note_ons = Vec::new();
    let mut note_offs = Vec::new();
    let mut tick = 0u64;
    for event in written.iter_track(0) {
        let event = event.expect("parse written track");
        tick += event.delta;
        match event.event {
            Event::Tempo(tempo) => {
                seen_tempo += 1;
                assert_eq!(tempo.tempo, 500_000);
                assert_eq!(tick, 0);
            }
            Event::ProgramChange(program) => {
                seen_program += 1;
                assert_eq!(program.program, 5);
                assert_eq!(tick, 0);
            }
            Event::NoteOn(note) => note_ons.push((tick, note.channel, note.key)),
            Event::NoteOff(note) => note_offs.push((tick, note.channel, note.key)),
            _ => {}
        }
    }

    assert_eq!(seen_tempo, 1);
    assert_eq!(seen_program, 1);
    assert_eq!(note_ons, vec![(0, 1, 77), (36, 0, 72)]);
    assert_eq!(note_offs, vec![(60, 1, 77), (144, 0, 72)]);
}

#[test]
fn process_midi_files_job_reports_status_and_finishes() {
    let dir = std::env::temp_dir().join(format!(
        "meridian-core-process-job-test-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    fs::create_dir_all(&dir).expect("create temp test dir");
    let midi = dir.join("input.mid");
    let output = dir.join("job_out.mid");
    write_toolkit_midi(
        &midi,
        96,
        vec![vec![
            Event::new_delta_tempo_event(0, 500_000),
            Event::new_delta_note_on_event(0, 0, 60, 100),
            Event::new_delta_note_off_event(96, 0, 60),
        ]],
    );

    let core = spawn_core();
    let event_rx = core.subscribe_events();
    let events = core
        .request(CoreCommand::StartProcessMidiFiles {
            selection: MidiFileSelection {
                inputs: vec![midi.clone(), midi],
            },
            output: output.clone(),
            config: MidiFileProcessingConfig::default(),
        })
        .expect("start midi process job");

    assert!(matches!(
        events.as_slice(),
        [CoreEvent::MidiProcessStatus {
            status: MidiProcessStatus::Running {
                total_inputs: 2,
                ..
            }
        }]
    ));

    let mut saw_progress = false;
    let mut saw_finished = false;
    for _ in 0..10 {
        let event = event_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("receive job event");
        match event {
            CoreEvent::MidiProcess { event } => match event {
                MidiProcessEvent::InputProgress {
                    processed_inputs, ..
                } => {
                    saw_progress |= processed_inputs > 0;
                }
                MidiProcessEvent::ProcessFinished {
                    output: finished_output,
                    input_count,
                    ..
                } => {
                    assert_eq!(finished_output, output);
                    assert_eq!(input_count, 2);
                    saw_finished = true;
                    break;
                }
                _ => {}
            },
            _ => {}
        }
    }

    assert!(saw_progress);
    assert!(saw_finished);
    assert!(output.exists());

    let status = core
        .request(CoreCommand::GetProcessMidiStatus)
        .expect("get midi process status");
    assert!(matches!(
        status.as_slice(),
        [CoreEvent::MidiProcessStatus {
            status: MidiProcessStatus::Idle
        }]
    ));
}

#[test]
fn scene_and_view_commands_are_separate() {
    let core = spawn_core();

    let default_scene = SceneLayout::default().scene;
    let events = core
        .request(CoreCommand::SetSceneConfig {
            scene: default_scene.clone(),
        })
        .expect("set scene config");
    assert!(matches!(
        events.as_slice(),
        [CoreEvent::StateSnapshot { state }] if state.scene == default_scene
    ));

    let events = core
        .request(CoreCommand::SetViewRange {
            seconds: 12.0,
            time_space: None,
        })
        .expect("set view range");
    assert!(matches!(
        events.as_slice(),
        [CoreEvent::StateSnapshot { state }] if (state.view_range - 12.0).abs() < f64::EPSILON
    ));
}

#[test]
fn tick_space_render_shows_staircase_notes_at_distinct_positions() {
    let midi = write_tempo_staircase_midi();
    let core = spawn_core();

    core.request(CoreCommand::LoadMidi { path: midi })
        .expect("load midi");
    core.request(CoreCommand::SetViewRange {
        seconds: 2.0,
        time_space: Some(DisplayTimeSpace::Tick),
    })
    .expect("set tick-space view range");
    core.request(CoreCommand::SetTime { time: 284.0 })
        .expect("set time");
    core.request(CoreCommand::SetViewport {
        width: 320,
        height: 180,
    })
    .expect("set viewport");

    let frame = core
        .render_frame(Some(320), Some(180))
        .expect("render frame");

    assert_eq!(frame.layout.time_space, DisplayTimeSpace::Tick);
    assert_eq!(frame.stats.visible_notes, 5);

    let mut notes = frame
        .scene
        .note_layer(meridian_core::render::SceneLayer::WhiteNotes)
        .iter()
        .chain(
            frame
                .scene
                .note_layer(meridian_core::render::SceneLayer::BlackNotes)
                .iter(),
        )
        .map(|note| (note.key, note.start, note.end))
        .collect::<Vec<_>>();
    notes.sort_by_key(|(key, _, _)| *key);

    let expected = [
        (60_u32, 0.008_327_f32),
        (61, 0.103_264),
        (62, 0.198_2),
        (63, 0.293_137),
        (64, 0.388_074),
    ];

    for ((actual_key, start, end), (expected_key, expected_start)) in notes.iter().zip(expected) {
        assert_eq!(*actual_key, expected_key);
        assert!(
            (*start - expected_start).abs() < 0.001,
            "key {actual_key}: expected start ~{expected_start}, got {start}"
        );
        assert!(
            ((*end - *start) - 0.189_873_5).abs() < 0.001,
            "key {actual_key}: expected visible length ~0.1898735, got {}",
            *end - *start
        );
    }
}
