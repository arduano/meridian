mod args;
mod output;
mod process_tools;

use meridian_core::{
    MeridianError,
    protocol::{
        MidiAnalysisJobStatus, MidiProcessEvent, MidiProcessStatus, ProtocolClient,
        ProtocolCommand, ProtocolEvent,
    },
};

use self::{
    args::{
        Cli, Command, DebugCommand, DebugPianoTrailClassicGeometryArgs, FrameStdoutArgs,
        InspectArgs, JsonArgs, MergeArgs, ProcessCommand, ProcessCommonArgs, RenderAudioArgs,
        RenderCommand, RenderVideoArgs,
    },
    output::print_json_to_stdout,
    process_tools::{
        ProcessTool, analysis_kinds, build_process_config, process_quantize_tool,
        process_range_select_tool, process_tempo_flatten_tool, process_tempo_scale_tool,
    },
};

pub fn run() -> Result<(), MeridianError> {
    let cli = <Cli as clap::Parser>::parse();
    match cli.command {
        Command::Stdio => crate::json_mode::serve_json(),
        Command::Json(JsonArgs { raw }) => crate::json_mode::run_one_json(raw.join(" ").trim()),
        Command::Analyze(args) => run_analyze(args),
        Command::Merge(args) => run_merge(args),
        Command::Inspect(args) => run_inspect(args),
        Command::Process {
            command: ProcessCommand::Select(args),
        } => run_process(
            ProcessTool::Select(process_range_select_tool(&args)),
            args.common,
        ),
        Command::Process {
            command: ProcessCommand::TempoFlatten(args),
        } => run_process(
            ProcessTool::Tempo(process_tempo_flatten_tool(&args)),
            args.common,
        ),
        Command::Process {
            command: ProcessCommand::TempoScale(args),
        } => run_process(
            ProcessTool::Tempo(process_tempo_scale_tool(&args)),
            args.common,
        ),
        Command::Process {
            command: ProcessCommand::Quantize(args),
        } => run_process(
            ProcessTool::Quantize(process_quantize_tool(&args)),
            args.common,
        ),
        Command::Render {
            command: RenderCommand::Frame(args),
        }
        | Command::FrameStdout(args) => run_frame_stdout(args),
        Command::Render {
            command: RenderCommand::Video(args),
        }
        | Command::RenderVideo(args) => run_render_video(args),
        Command::Render {
            command: RenderCommand::Audio(args),
        }
        | Command::RenderAudio(args) => run_render_audio(args),
        Command::Bench(args) | Command::Benchmark(args) => run_benchmark(args),
        Command::Debug {
            command: DebugCommand::PianoTrailClassicGeometry(args),
        }
        | Command::DebugPianoTrailClassicGeometry(args) => run_debug_geometry(args),
    }
}

fn run_analyze(args: args::AnalyzeArgs) -> Result<(), MeridianError> {
    let kinds = analysis_kinds(&args);
    let bucket_count = args.buckets;
    let client = ProtocolClient::spawn();

    let parsed_events = client.request(ProtocolCommand::LoadParsedMidi {
        path: args.midi.clone(),
    })?;
    let parsed_midi_id = parsed_events
        .events
        .iter()
        .find_map(|event| match event {
            ProtocolEvent::ParsedMidiLoaded { parsed_midi_id, .. } => Some(*parsed_midi_id),
            _ => None,
        })
        .ok_or_else(|| MeridianError::Protocol("missing parsed midi id".into()))?;

    let status_events = client.request(ProtocolCommand::StartMidiAnalysisJob {
        parsed_midi_id,
        kinds,
        bucket_count,
    })?;
    let job_id = match status_events.events.as_slice() {
        [
            ProtocolEvent::MidiAnalysisJobStatus {
                status: MidiAnalysisJobStatus::Running { job_id, .. },
            },
        ] => *job_id,
        [
            ProtocolEvent::MidiAnalysisJobStatus {
                status: MidiAnalysisJobStatus::Finished { result, .. },
            },
        ] => {
            print_json_to_stdout(result, args.pretty)?;
            let _ = client.shutdown();
            return Ok(());
        }
        [
            ProtocolEvent::MidiAnalysisJobStatus {
                status: MidiAnalysisJobStatus::Failed { message, .. },
            },
        ] => return Err(MeridianError::Protocol(message.clone())),
        other => {
            return Err(MeridianError::Protocol(format!(
                "unexpected analysis start response: {other:?}"
            )));
        }
    };

    let result = loop {
        let event = client.recv()?;
        if let ProtocolEvent::MidiAnalysisJob { event } = event {
            match event {
                meridian_core::protocol::MidiAnalysisJobEvent::Progress {
                    job_id: event_job_id,
                    progress,
                    status,
                } if event_job_id == job_id => {
                    eprintln!("analysis: {:>3.0}% {status}", progress as f64 * 100.0);
                }
                meridian_core::protocol::MidiAnalysisJobEvent::Finished {
                    job_id: event_job_id,
                    result,
                } if event_job_id == job_id => break Ok(result),
                meridian_core::protocol::MidiAnalysisJobEvent::Failed {
                    job_id: event_job_id,
                    message,
                } if event_job_id == job_id => break Err(MeridianError::Protocol(message)),
                _ => {}
            }
        }
    };

    let _ = client.shutdown();
    print_json_to_stdout(&result?, args.pretty)
}

fn run_process(tool: ProcessTool, common: ProcessCommonArgs) -> Result<(), MeridianError> {
    let config = build_process_config(tool);
    let client = ProtocolClient::spawn();
    let status_events = client.request(ProtocolCommand::StartProcessMidiFile {
        input: common.input.clone(),
        output: common.output.clone(),
        config,
    })?;

    let job_id = match status_events.events.as_slice() {
        [
            ProtocolEvent::MidiProcessStatus {
                status: MidiProcessStatus::Running { job_id, .. },
            },
        ] => {
            eprintln!("process: started job {job_id:?}");
            *job_id
        }
        [
            ProtocolEvent::MidiProcessStatus {
                status: MidiProcessStatus::Idle,
            },
        ] => {
            return Err(MeridianError::Protocol(
                "midi processing did not enter a running state".into(),
            ));
        }
        other => {
            return Err(MeridianError::Protocol(format!(
                "unexpected process start response: {other:?}"
            )));
        }
    };

    let result = loop {
        let event = client.recv()?;
        if let ProtocolEvent::MidiProcess { event } = event {
            match event {
                MidiProcessEvent::ProcessStarted {
                    job_id: event_job_id,
                    input,
                    ..
                } if event_job_id == job_id => {
                    eprintln!("process: {}", input.display());
                }
                MidiProcessEvent::Progress {
                    job_id: event_job_id,
                    progress_percent,
                    label,
                } if event_job_id == job_id => {
                    eprintln!("process: {progress_percent:>3}% {label}");
                }
                MidiProcessEvent::ProcessFinished {
                    job_id: event_job_id,
                    ..
                } if event_job_id == job_id => break Ok(event),
                MidiProcessEvent::ProcessFailed {
                    job_id: event_job_id,
                    message,
                    ..
                } if event_job_id == job_id => break Err(MeridianError::Protocol(message)),
                MidiProcessEvent::ProcessCancelled {
                    job_id: event_job_id,
                    ..
                } if event_job_id == job_id => {
                    break Err(MeridianError::Cancelled(
                        "midi processing was cancelled".into(),
                    ));
                }
                _ => {}
            }
        }
    };

    let _ = client.shutdown();
    print_json_to_stdout(&result?, common.pretty)
}

fn run_merge(args: MergeArgs) -> Result<(), MeridianError> {
    let client = ProtocolClient::spawn();
    let response = client.request(ProtocolCommand::MergeMidiFiles {
        inputs: args.inputs,
        output: args.output,
        config: meridian_core::midi::MidiFilesMergeConfig::default(),
    })?;
    assert_no_protocol_error(&response.events)?;
    let _ = client.shutdown();
    print_json_to_stdout(&response.events, args.pretty)
}

fn run_inspect(args: InspectArgs) -> Result<(), MeridianError> {
    let client = ProtocolClient::spawn();
    let response = client.request(ProtocolCommand::InspectMidiFiles { paths: args.paths })?;
    assert_no_protocol_error(&response.events)?;
    let _ = client.shutdown();
    print_json_to_stdout(&response.events, args.pretty)
}

fn run_frame_stdout(args: FrameStdoutArgs) -> Result<(), MeridianError> {
    crate::frame_stdout::run(
        &args.midi,
        args.format,
        args.time,
        args.view_range,
        args.time_space,
        args.first_key,
        args.last_key,
        args.width,
        args.height,
        args.renderer,
    )
}

fn run_benchmark(args: args::BenchmarkArgs) -> Result<(), MeridianError> {
    crate::benchmark::run(
        &args.midi,
        args.time,
        args.view_range,
        args.time_space,
        args.first_key,
        args.last_key,
        args.width,
        args.height,
        args.renderer,
        args.iterations,
        args.warmup,
    )
}

fn run_render_video(args: RenderVideoArgs) -> Result<(), MeridianError> {
    crate::render_video::run(
        &args.midi,
        &args.output,
        args.fps,
        args.container,
        args.width,
        args.height,
        args.view_range,
        args.time_space,
        args.first_key,
        args.last_key,
        args.renderer,
        args.rgb_mode,
        args.export_alpha_mask,
        args.ffmpeg_flags.as_deref(),
    )
}

fn run_render_audio(args: RenderAudioArgs) -> Result<(), MeridianError> {
    crate::render_audio::run(
        &args.midi,
        &args.output,
        args.format,
        args.sample_rate,
        args.channels,
        !args.no_limiter,
        &args.soundfont,
    )
}

fn run_debug_geometry(args: DebugPianoTrailClassicGeometryArgs) -> Result<(), MeridianError> {
    crate::debug_piano_trail_classic::run(
        args.first_key,
        args.last_key,
        args.width,
        args.height,
        args.scene_json.as_deref(),
    )
}

fn assert_no_protocol_error(events: &[ProtocolEvent]) -> Result<(), MeridianError> {
    if let Some(ProtocolEvent::Error { code, message }) = events
        .iter()
        .find(|event| matches!(event, ProtocolEvent::Error { .. }))
    {
        return Err(MeridianError::Protocol(format!("{code:?}: {message}")));
    }
    Ok(())
}
