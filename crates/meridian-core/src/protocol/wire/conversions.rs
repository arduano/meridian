use super::*;

impl From<ProtocolAudioRenderConfig> for crate::audio::AudioRenderConfig {
    fn from(value: ProtocolAudioRenderConfig) -> Self {
        Self {
            midi_path: Some(value.midi_path),
            audio: None,
            output: value.output,
            sample_rate: value.sample_rate,
            channels: value.channels,
            use_limiter: value.use_limiter,
            format: value.format,
            ffmpeg_args: value.ffmpeg_args,
            soundfonts: value.soundfonts,
        }
    }
}

impl From<ProtocolVideoRenderConfig> for crate::protocol::VideoRenderConfig {
    fn from(value: ProtocolVideoRenderConfig) -> Self {
        let scene = value.scene.unwrap_or_else(|| {
            let mut layout = SceneLayout::default();
            if let Some(renderer) = value.renderer {
                layout.set_renderer_kind(renderer);
            }
            layout.scene
        });
        Self {
            midi_path: Some(value.midi_path),
            output: value.output,
            fps: value.fps,
            width: value.width,
            height: value.height,
            scene: Some(scene),
            view_range: value.view_range,
            time_space: value.time_space,
            first_key: value.first_key,
            last_key: value.last_key,
            ffmpeg_args: value.ffmpeg_args,
            export: value.export,
            audio: value.audio,
        }
    }
}

impl From<crate::protocol::StateSnapshot> for ProtocolStateSnapshot {
    fn from(value: crate::protocol::StateSnapshot) -> Self {
        Self {
            midi_path: value.midi_path,
            current_time: value.current_time,
            playing: value.playing,
            scene: value.scene,
            view_range: value.view_range,
            time_space: value.time_space,
            first_key: value.first_key,
            last_key: value.last_key,
            viewport_width: value.viewport_width,
            viewport_height: value.viewport_height,
        }
    }
}

impl From<ProtocolCommand> for CoreCommand {
    fn from(value: ProtocolCommand) -> Self {
        match value {
            ProtocolCommand::LoadParsedMidi { path } => Self::LoadParsedMidi { path },
            ProtocolCommand::InspectMidiFiles { paths } => Self::InspectMidiFiles { paths },
            ProtocolCommand::LoadMidi { path } => Self::LoadMidi { path },
            ProtocolCommand::LoadAudioMidi { path } => Self::LoadAudioMidi { path },
            ProtocolCommand::StartMidiAnalysisJob {
                parsed_midi_id,
                kinds,
                bucket_count,
            } => Self::StartMidiAnalysisJob {
                parsed_midi_id,
                kinds,
                bucket_count,
            },
            ProtocolCommand::StartProcessMidiFile {
                input,
                output,
                config,
            } => Self::StartProcessMidiFile {
                input,
                output,
                config,
            },
            ProtocolCommand::MergeMidiFiles {
                inputs,
                output,
                config,
            } => Self::MergeMidiFiles {
                inputs,
                output,
                config,
            },
            ProtocolCommand::CancelMidiFileProcess => Self::CancelMidiFileProcess,
            ProtocolCommand::GetMidiFileProcessStatus => Self::GetMidiFileProcessStatus,
            ProtocolCommand::StartRenderAudio { config } => Self::StartRenderAudio {
                config: config.into(),
            },
            ProtocolCommand::CancelRenderAudio => Self::CancelRenderAudio,
            ProtocolCommand::GetRenderAudioStatus => Self::GetRenderAudioStatus,
            ProtocolCommand::SetTime { time } => Self::SetTime { time },
            ProtocolCommand::TickProjectorPhysics { delta_seconds } => {
                Self::TickProjectorPhysics { delta_seconds }
            }
            ProtocolCommand::ResetProjectorPhysics => Self::ResetProjectorPhysics,
            ProtocolCommand::StepTime { delta } => Self::StepTime { delta },
            ProtocolCommand::SetPlaying { playing } => Self::SetPlaying { playing },
            ProtocolCommand::TogglePlaying => Self::TogglePlaying,
            ProtocolCommand::SetSceneConfig { scene } => Self::SetSceneConfig { scene },
            ProtocolCommand::SetViewRange {
                seconds,
                time_space,
            } => Self::SetViewRange {
                seconds,
                time_space,
            },
            ProtocolCommand::SetKeyRange {
                first_key,
                last_key,
            } => Self::SetKeyRange {
                first_key,
                last_key,
            },
            ProtocolCommand::SetViewport { width, height } => Self::SetViewport { width, height },
            ProtocolCommand::SaveFrame {
                output,
                format,
                viewport_width,
                viewport_height,
                export,
            } => Self::SaveFrame {
                output,
                format,
                viewport_width,
                viewport_height,
                export,
            },
            ProtocolCommand::StartRenderVideo { config } => Self::StartRenderVideo {
                config: config.into(),
            },
            ProtocolCommand::CancelRenderVideo => Self::CancelRenderVideo,
            ProtocolCommand::GetRenderVideoStatus => Self::GetRenderVideoStatus,
            ProtocolCommand::Shutdown => Self::Shutdown,
        }
    }
}

impl TryFrom<CoreEvent> for ProtocolEvent {
    type Error = UnsupportedProtocolEvent;

    fn try_from(value: CoreEvent) -> Result<Self, UnsupportedProtocolEvent> {
        match value {
            CoreEvent::ParsedMidiLoaded {
                parsed_midi_id,
                path,
            } => Ok(Self::ParsedMidiLoaded {
                parsed_midi_id,
                path,
            }),
            CoreEvent::MidiFilesInspected { inspections } => {
                Ok(Self::MidiFilesInspected { inspections })
            }
            CoreEvent::StateSnapshot { state } => Ok(Self::StateSnapshot {
                state: state.into(),
            }),
            CoreEvent::MidiLoaded { path, .. } => Ok(Self::MidiLoaded { path }),
            CoreEvent::MidiFileProcessed {
                input,
                output,
                output_track_count,
                output_ppq,
                total_events,
            } => Ok(Self::MidiFileProcessed {
                input,
                output,
                output_track_count,
                output_ppq,
                total_events,
            }),
            CoreEvent::MidiFilesMerged {
                output,
                input_count,
                output_track_count,
                output_ppq,
                total_events,
            } => Ok(Self::MidiFilesMerged {
                output,
                input_count,
                output_track_count,
                output_ppq,
                total_events,
            }),
            CoreEvent::MidiAnalysisJob { event } => Ok(Self::MidiAnalysisJob { event }),
            CoreEvent::MidiAnalysisJobStatus { status } => {
                Ok(Self::MidiAnalysisJobStatus { status })
            }
            CoreEvent::MidiProcess { event } => Ok(Self::MidiProcess { event }),
            CoreEvent::MidiProcessStatus { status } => Ok(Self::MidiProcessStatus { status }),
            CoreEvent::AudioRender { event } => Ok(Self::AudioRender { event }),
            CoreEvent::AudioRenderStatus { status } => Ok(Self::AudioRenderStatus { status }),
            CoreEvent::FrameSaved {
                output,
                format,
                stats,
                bytes_written,
                exports,
                ..
            } => Ok(Self::FrameSaved {
                output,
                format,
                stats,
                bytes_written,
                exports,
            }),
            CoreEvent::VideoRender { event } => Ok(Self::VideoRender { event }),
            CoreEvent::VideoRenderStatus { status } => Ok(Self::VideoRenderStatus { status }),
            CoreEvent::Error { code, message } => Ok(Self::Error { code, message }),
            CoreEvent::ShutdownComplete => Ok(Self::ShutdownComplete),
            _ => Err(UnsupportedProtocolEvent),
        }
    }
}
