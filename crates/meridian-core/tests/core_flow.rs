mod support;

use std::{
    fs,
    time::{Duration, Instant},
};

use meridian_core::{
    PROTOCOL_VERSION,
    midi::{
        KeyMapEntry, KeyMapTool, MidiFileProcessingConfig, MidiFilesMergeConfig, MidiModifierTool,
        RangeEdgeBehavior, RangeSelectTool, analysis::MidiAnalysisKind,
    },
    protocol::{
        CoreCommand, CoreEvent, JsonRequest, JsonResponse, MidiProcessEvent, MidiProcessStatus,
    },
    render::{DisplayTimeSpace, SceneLayout},
    spawn_core,
};
use midi_toolkit::{
    events::{Event, MIDIEvent, TextEvent, TextEventKind},
    io::MIDIFile as ToolkitMidiFile,
    sequence::event::Delta,
};

#[test]
fn stateful_core_projects_a_frame() {
    let midi = support::write_test_midi();
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
    let midi = support::write_test_midi();
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
    let midi = support::write_test_midi();
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
    let midi = support::write_test_midi();
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
fn process_midi_file_applies_range_select_tool() {
    let dir = support::temp_dir("meridian-core-process-test");
    let midi = dir.join("input.mid");
    let output = dir.join("out.mid");

    support::write_toolkit_midi(
        &midi,
        96,
        vec![
            vec![
                Event::new_delta_program_change_event(0, 0, 5),
                Event::new_delta_note_on_event(48, 0, 60, 100),
                Event::new_delta_note_off_event(48, 0, 60),
            ],
            vec![
                Event::new_delta_note_on_event(0, 1, 65, 90),
                Event::new_delta_note_off_event(72, 1, 65),
            ],
        ],
    );

    let core = spawn_core();
    let config = MidiFileProcessingConfig {
        tool: MidiModifierTool::RangeSelect(RangeSelectTool {
            start_ticks: 24,
            end_ticks: 120,
            offset_ticks: Some(0),
            track_select: None,
            preserve_system_events: false,
            edge_behavior: RangeEdgeBehavior::Trim,
        }),
    };

    let events = core
        .request(CoreCommand::ProcessMidiFile {
            input: midi.clone(),
            output: output.clone(),
            config,
        })
        .expect("process midi file");

    assert!(matches!(
        events.as_slice(),
        [CoreEvent::MidiFileProcessed {
            input: processed_input,
            output: processed_output,
            output_track_count: 2,
            output_ppq: 96,
            ..
        }] if processed_input == &midi && processed_output == &output
    ));

    let written = ToolkitMidiFile::open_in_ram(&output, None).expect("open output midi");
    assert_eq!(written.ppq(), 96);
    assert_eq!(written.track_count(), 2);

    let mut track0_events = Vec::new();
    let mut tick = 0u64;
    for event in written.iter_track(0).expect("open written track 0") {
        let event = event.expect("parse written track 0");
        tick += event.delta;
        if let Some(key) = event.event.key() {
            track0_events.push((tick, key));
        }
    }

    let mut track1_events = Vec::new();
    tick = 0;
    for event in written.iter_track(1).expect("open written track 1") {
        let event = event.expect("parse written track 1");
        tick += event.delta;
        if let Some(key) = event.event.key() {
            track1_events.push((tick, key));
        }
    }

    assert_eq!(track0_events, vec![(24, 60), (72, 60)]);
    assert_eq!(track1_events, vec![(0, 65), (48, 65)]);
}

#[test]
fn process_midi_file_applies_key_map_tool() {
    let dir = support::temp_dir("meridian-core-process-key-map-test");
    let midi = dir.join("input.mid");
    let output = dir.join("out.mid");

    support::write_toolkit_midi(
        &midi,
        96,
        vec![vec![
            Event::new_delta_note_on_event(0, 0, 60, 100),
            Event::new_delta_note_off_event(48, 0, 60),
            Event::new_delta_note_on_event(0, 0, 61, 100),
            Event::new_delta_note_off_event(48, 0, 61),
        ]],
    );

    let core = spawn_core();
    let config = MidiFileProcessingConfig {
        tool: MidiModifierTool::KeyMap(KeyMapTool {
            mappings: vec![KeyMapEntry { from: 60, to: 72 }],
            fold_to_range: None,
            drop_unmapped: true,
        }),
    };

    let events = core
        .request(CoreCommand::ProcessMidiFile {
            input: midi.clone(),
            output: output.clone(),
            config,
        })
        .expect("process midi file");

    assert!(matches!(
        events.as_slice(),
        [CoreEvent::MidiFileProcessed {
            output_track_count: 1,
            ..
        }]
    ));

    let written = ToolkitMidiFile::open_in_ram(&output, None).expect("open output midi");
    let keys = written
        .iter_track(0)
        .expect("open output track")
        .filter_map(|event| event.expect("parse output event").event.key())
        .collect::<Vec<_>>();

    assert_eq!(keys, vec![72, 72]);
}

#[test]
fn merge_midi_files_trims_and_writes_output() {
    let dir = support::temp_dir("meridian-core-merge-test");
    let midi_a = dir.join("a.mid");
    let midi_b = dir.join("b.mid");
    let output = dir.join("merged.mid");

    support::write_toolkit_midi(
        &midi_a,
        96,
        vec![vec![
            Event::new_delta_tempo_event(0, 600_000),
            Event::new_delta_program_change_event(0, 0, 5),
            Event::new_delta_note_on_event(48, 0, 60, 100),
            Event::new_delta_note_off_event(96, 0, 60),
        ]],
    );
    support::write_toolkit_midi(
        &midi_b,
        96,
        vec![vec![
            Event::new_delta_note_on_event(0, 1, 65, 90),
            Event::new_delta_note_off_event(96, 1, 65),
        ]],
    );

    let core = spawn_core();

    let events = core
        .request(CoreCommand::MergeMidiFiles {
            inputs: vec![midi_a.clone(), midi_b.clone()],
            output: output.clone(),
            config: MidiFilesMergeConfig::default(),
        })
        .expect("merge midi files");

    assert!(matches!(
        events.as_slice(),
        [CoreEvent::MidiFilesMerged {
            output: merged_output,
            input_count: 2,
            output_track_count: 2,
            output_ppq: 96,
            ..
        }] if merged_output == &output
    ));

    let written = ToolkitMidiFile::open_in_ram(&output, None).expect("open output midi");
    assert_eq!(written.ppq(), 96);
    assert_eq!(written.track_count(), 2);

    let track_a = written
        .iter_track(0)
        .expect("open written track a")
        .map(|event| event.expect("parse written track a").event)
        .collect::<Vec<_>>();
    let track_b = written
        .iter_track(1)
        .expect("open written track b")
        .map(|event| event.expect("parse written track b").event)
        .collect::<Vec<_>>();

    assert!(matches!(
        track_a.as_slice(),
        [
            Event::Tempo(_),
            Event::ProgramChange(_),
            Event::NoteOn(_),
            Event::NoteOff(_)
        ]
    ));
    assert!(matches!(
        track_b.as_slice(),
        [Event::NoteOn(_), Event::NoteOff(_)]
    ));
}

#[test]
fn inspect_midi_files_reports_basic_merge_metadata() {
    let dir = support::temp_dir("meridian-core-inspect-test");
    let midi = dir.join("inspect.mid");
    support::write_toolkit_midi(
        &midi,
        96,
        vec![
            vec![
                Event::new_delta_tempo_event(0, 500_000),
                Event::new_delta_note_on_event(24, 0, 60, 100),
                Event::new_delta_note_off_event(48, 0, 60),
            ],
            vec![Delta::new(
                0,
                Event::Text(Box::new(TextEvent {
                    kind: TextEventKind::TrackName,
                    bytes: b"Strings".to_vec(),
                })),
            )],
        ],
    );

    let core = spawn_core();
    let events = core
        .request(CoreCommand::InspectMidiFiles {
            paths: vec![midi.clone()],
        })
        .expect("inspect midi files");

    match events.as_slice() {
        [CoreEvent::MidiFilesInspected { inspections }] => {
            assert_eq!(inspections.len(), 1);
            let inspection = &inspections[0];
            assert_eq!(inspection.path, midi);
            assert_eq!(inspection.actual_track_count, 2);
            assert_eq!(inspection.tempo_event_count, 1);
            assert_eq!(inspection.track_name_event_count, 1);
            assert_eq!(inspection.total_notes, 1);
            assert_eq!(inspection.ticks_per_quarter, Some(96));
            assert!(inspection.error.is_none());
        }
        other => panic!("unexpected inspect response: {other:?}"),
    }
}

#[test]
fn process_midi_file_job_reports_status_and_finishes() {
    let dir = support::temp_dir("meridian-core-process-job-test");
    let midi = dir.join("input.mid");
    let output = dir.join("job_out.mid");
    support::write_toolkit_midi(
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
        .request(CoreCommand::StartProcessMidiFile {
            input: midi.clone(),
            output: output.clone(),
            config: MidiFileProcessingConfig {
                tool: MidiModifierTool::KeyMap(KeyMapTool::default()),
            },
        })
        .expect("start midi process job");

    assert!(matches!(
        events.as_slice(),
        [CoreEvent::MidiProcessStatus {
            status: MidiProcessStatus::Running {
                input: running_input,
                output: running_output,
                ..
            }
        }] if running_input == &midi && running_output == &output
    ));

    let mut saw_started = false;
    let mut saw_finished = false;
    for _ in 0..10 {
        let event = event_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("receive job event");
        match event {
            CoreEvent::MidiProcess { event } => match event {
                MidiProcessEvent::ProcessStarted {
                    input: started_input,
                    output: started_output,
                    ..
                } => {
                    assert_eq!(started_input, midi);
                    assert_eq!(started_output, output);
                    saw_started = true;
                }
                MidiProcessEvent::ProcessFinished {
                    input: finished_input,
                    output: finished_output,
                    ..
                } => {
                    assert_eq!(finished_input, midi);
                    assert_eq!(finished_output, output);
                    saw_finished = true;
                    break;
                }
                _ => {}
            },
            _ => {}
        }
    }

    assert!(saw_started);
    assert!(saw_finished);
    assert!(output.exists());

    let status = core
        .request(CoreCommand::GetMidiFileProcessStatus)
        .expect("get midi process status");
    assert!(matches!(
        status.as_slice(),
        [CoreEvent::MidiProcessStatus {
            status: MidiProcessStatus::Idle
        }]
    ));
}

#[test]
fn midi_load_progress_reports_materialization_counts() {
    let midi = support::write_test_midi();
    let core = spawn_core();
    let event_rx = core.subscribe_events();

    core.request(CoreCommand::LoadDisplayMidi { path: midi.clone() })
        .expect("load display midi");

    let deadline = Instant::now() + Duration::from_secs(1);
    while Instant::now() < deadline {
        let Ok(event) = event_rx.recv_timeout(Duration::from_millis(50)) else {
            continue;
        };
        match event {
            CoreEvent::MidiLoadProgress { path, progress, .. } if path == midi => {
                if progress.completed_events.is_some_and(|value| value > 0)
                    && progress.completed_notes.is_some_and(|value| value > 0)
                {
                    return;
                }
            }
            _ => {}
        }
    }

    panic!("did not observe count-bearing midi load progress");
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
    let midi = support::write_tempo_staircase_midi();
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

#[test]
fn video_render_smokes_when_ffmpeg_is_available() {
    if !support::ffmpeg_available() {
        eprintln!("skipping video smoke test because ffmpeg is unavailable");
        return;
    }

    let midi = support::write_test_midi();
    let dir = support::temp_dir("meridian-core-video-test");
    let output = dir.join("video.mp4");
    let core = spawn_core();
    let event_rx = core.subscribe_events();

    let events = core
        .request(CoreCommand::StartRenderVideo {
            config: meridian_core::protocol::VideoRenderConfig {
                midi_path: Some(midi),
                output: output.clone(),
                fps: 4.0,
                width: 160,
                height: 90,
                scene: Some(SceneLayout::default().scene),
                view_range: Some(2.0),
                time_space: Some(DisplayTimeSpace::Tick),
                first_key: None,
                last_key: None,
                ffmpeg_args: vec!["-y".to_string()],
            },
        })
        .expect("start video render");

    assert!(matches!(
        events.as_slice(),
        [CoreEvent::VideoRenderStatus {
            status: meridian_core::protocol::VideoRenderStatus::Running { .. }
        }]
    ));

    let mut saw_finished = false;
    for _ in 0..60 {
        match event_rx.recv_timeout(Duration::from_secs(2)) {
            Ok(CoreEvent::VideoRender {
                event:
                    meridian_core::protocol::VideoRenderEvent::RenderFinished {
                        output: finished_output,
                        ..
                    },
            }) => {
                assert_eq!(finished_output, output);
                saw_finished = true;
                break;
            }
            Ok(CoreEvent::VideoRender {
                event: meridian_core::protocol::VideoRenderEvent::RenderFailed { message },
            }) => {
                panic!("video render failed: {message}");
            }
            Ok(_) => {}
            Err(error) => panic!("timed out waiting for video render to finish: {error}"),
        }
    }

    assert!(saw_finished, "video render did not finish");
    assert!(output.exists(), "expected video output to exist");
    assert!(fs::metadata(&output).expect("video output metadata").len() > 0);
}
