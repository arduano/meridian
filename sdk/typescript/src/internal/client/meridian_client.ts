import { midiTools } from "../../helpers.ts";
import type {
  AudioRenderEvent,
  ChangePpqTool,
  ChannelRemapTool,
  ControlChangeTool,
  ExtractTrackTool,
  HumanizeTool,
  KeyMapTool,
  MetaTextTool,
  MidiAnalysisData,
  MidiFilesMergeConfig,
  MidiFilesMergedEvent,
  MidiLoadedEvent,
  MidiModifierTool,
  MidiProcessEvent,
  NoteLengthTool,
  ParsedMidiId,
  PitchBendTool,
  ProgramTool,
  ProtocolVideoRenderConfig,
  QuantizeTool,
  RangeSelectTool,
  SharedMetadataTrackTool,
  SdkAudioRenderConfig,
  SysexTool,
  TempoPoint,
  TimeWarpTool,
  TrackRouteTool,
  VelocityMapTool,
  VideoRenderEvent,
} from "../../protocol.ts";
import type { DeepPartial } from "../../helpers.ts";
import type {
  AudioRenderOptions,
  MidiAnalysisOptions,
  MidiMergeOptions,
  MidiModificationOptions,
  MidiToolTaskOptions,
  StartAnalysisForFileOptions,
  VideoRenderOptions,
} from "./options.ts";
import {
  AudioRenderJobHandle,
  MidiAnalysisJobHandle,
  MidiProcessJobHandle,
  VideoRenderJobHandle,
} from "./handles.ts";
import { inferRendererFromScene, type ToolConfig } from "./internal.ts";
import {
  AudioRenderTask,
  MidiAnalysisTask,
  MidiProcessTask,
  VideoRenderTask,
} from "./tasks.ts";
import type { MeridianProtocolClient } from "./protocol_client.ts";
import { MeridianSubprocessError, requireEvent } from "./internal.ts";

export class MeridianClient {
  readonly protocol: MeridianProtocolClient;
  readonly resources = {
    loadParsedMidi: (path: string): Promise<ParsedMidiId> =>
      this.loadParsedMidi(path),
    loadAudioMidi: (path: string): Promise<MidiLoadedEvent> =>
      this.loadAudioMidi(path),
  };
  readonly audio = {
    render: (options: AudioRenderOptions): AudioRenderTask =>
      new AudioRenderTask(this, options),
  };
  readonly video = {
    render: (options: VideoRenderOptions): VideoRenderTask =>
      new VideoRenderTask(this, options),
  };
  readonly modification = {
    apply: (
      tool: MidiModifierTool,
      options: MidiModificationOptions,
    ): MidiProcessTask => this.midi({ ...options, tool }),
    rangeSelect: (
      options: MidiModificationOptions & Partial<ToolConfig<RangeSelectTool>>,
    ): MidiProcessTask =>
      this.#withTool(options, (tool) => midiTools.rangeSelect(tool)),
    tempoMap: {
      flatten: (
        tempo: number,
        options: MidiModificationOptions,
      ): MidiProcessTask =>
        this.midi({ ...options, tool: midiTools.tempoMap.flatten(tempo) }),
      scaleBpm: (
        factor: number,
        options: MidiModificationOptions,
      ): MidiProcessTask =>
        this.midi({ ...options, tool: midiTools.tempoMap.scaleBpm(factor) }),
      replace: (
        points: TempoPoint[],
        destination:
          | Parameters<typeof midiTools.tempoMap.replace>[1]
          | undefined,
        options: MidiModificationOptions,
      ): MidiProcessTask =>
        this.midi({
          ...options,
          tool: midiTools.tempoMap.replace(points, destination),
        }),
    },
    timeWarp: (
      options: MidiModificationOptions & Partial<ToolConfig<TimeWarpTool>>,
    ): MidiProcessTask =>
      this.#withTool(options, (tool) => midiTools.timeWarp(tool)),
    channelRemap: (
      options: MidiModificationOptions & Partial<ToolConfig<ChannelRemapTool>>,
    ): MidiProcessTask =>
      this.#withTool(options, (tool) => midiTools.channelRemap(tool)),
    trackRoute: {
      collapseAll: (options: MidiModificationOptions): MidiProcessTask =>
        this.midi({ ...options, tool: midiTools.trackRoute.collapseAll() }),
      splitByChannel: (options: MidiModificationOptions): MidiProcessTask =>
        this.midi({ ...options, tool: midiTools.trackRoute.splitByChannel() }),
      map: (
        mappings: Parameters<typeof midiTools.trackRoute.map>[0],
        options: MidiModificationOptions,
      ): MidiProcessTask =>
        this.midi({ ...options, tool: midiTools.trackRoute.map(mappings) }),
    },
    program: (
      options: MidiModificationOptions & Partial<ToolConfig<ProgramTool>>,
    ): MidiProcessTask =>
      this.#withTool(options, (tool) => midiTools.program(tool)),
    controlChange: (
      options: MidiModificationOptions & Partial<ToolConfig<ControlChangeTool>>,
    ): MidiProcessTask =>
      this.#withTool(options, (tool) => midiTools.controlChange(tool)),
    pitchBend: (
      options: MidiModificationOptions & Partial<ToolConfig<PitchBendTool>>,
    ): MidiProcessTask =>
      this.#withTool(options, (tool) => midiTools.pitchBend(tool)),
    velocityMap: {
      scale: (
        scale: number,
        options: MidiModificationOptions,
      ): MidiProcessTask =>
        this.midi({ ...options, tool: midiTools.velocityMap.scale(scale) }),
      gamma: (
        gamma: number,
        options: MidiModificationOptions,
      ): MidiProcessTask =>
        this.midi({ ...options, tool: midiTools.velocityMap.gamma(gamma) }),
      polyline: (
        points: Parameters<typeof midiTools.velocityMap.polyline>[0],
        options: MidiModificationOptions,
      ): MidiProcessTask =>
        this.midi({ ...options, tool: midiTools.velocityMap.polyline(points) }),
    },
    changePpq: (
      ppq: number,
      options: MidiModificationOptions,
    ): MidiProcessTask =>
      this.midi({ ...options, tool: midiTools.changePpq({ ppq }) }),
    extractTrack: (
      trackIndex: number,
      options: MidiModificationOptions,
    ): MidiProcessTask =>
      this.midi({
        ...options,
        tool: midiTools.extractTrack({ track_index: trackIndex }),
      }),
    noteLength: (
      options: MidiModificationOptions & Partial<ToolConfig<NoteLengthTool>>,
    ): MidiProcessTask =>
      this.#withTool(options, (tool) => midiTools.noteLength(tool)),
    quantize: (
      options: MidiModificationOptions & Partial<ToolConfig<QuantizeTool>>,
    ): MidiProcessTask =>
      this.#withTool(options, (tool) => midiTools.quantize(tool)),
    humanize: (
      options: MidiModificationOptions & Partial<ToolConfig<HumanizeTool>>,
    ): MidiProcessTask =>
      this.#withTool(options, (tool) => midiTools.humanize(tool)),
    keyMap: (
      options: MidiModificationOptions & Partial<ToolConfig<KeyMapTool>>,
    ): MidiProcessTask =>
      this.#withTool(options, (tool) => midiTools.keyMap(tool)),
    metaText: (
      options: MidiModificationOptions & Partial<ToolConfig<MetaTextTool>>,
    ): MidiProcessTask =>
      this.#withTool(options, (tool) => midiTools.metaText(tool)),
    sharedMetadataTrack: (
      options:
        & MidiModificationOptions
        & Partial<ToolConfig<SharedMetadataTrackTool>>,
    ): MidiProcessTask =>
      this.#withTool(options, (tool) => midiTools.sharedMetadataTrack(tool)),
    sysex: (
      options: MidiModificationOptions & Partial<ToolConfig<SysexTool>>,
    ): MidiProcessTask =>
      this.#withTool(options, (tool) => midiTools.sysex(tool)),
  };
  readonly merge = {
    midiFiles: (options: MidiMergeOptions): Promise<MidiFilesMergedEvent> =>
      this.mergeMidiFiles(options),
  };

  constructor(protocol: MeridianProtocolClient) {
    this.protocol = protocol;
  }

  analysis(
    midiPath: string,
    options: MidiAnalysisOptions = {},
  ): MidiAnalysisTask {
    return new MidiAnalysisTask(this, midiPath, options);
  }

  midi(options: MidiToolTaskOptions): MidiProcessTask {
    return new MidiProcessTask(this, options);
  }

  async close(): Promise<void> {
    await this.protocol.close();
  }

  #withTool<T extends MidiModifierTool>(
    options: MidiModificationOptions & Partial<ToolConfig<T>>,
    build: (config: Partial<ToolConfig<T>>) => T,
  ): MidiProcessTask {
    const { input, output, onEvent, ...toolConfig } = options;
    return this.midi({
      input,
      output,
      ...(onEvent ? { onEvent } : {}),
      tool: build(toolConfig as unknown as Partial<ToolConfig<T>>),
    });
  }

  async startAnalysis(
    options: StartAnalysisForFileOptions,
  ): Promise<MidiAnalysisJobHandle> {
    const parsedMidiId = await this.resources.loadParsedMidi(options.midiPath);
    const events = await this.protocol.request({
      type: "start_midi_analysis_job",
      parsed_midi_id: parsedMidiId,
      kinds: options.kinds ?? [],
      bucket_count: options.bucketCount ?? null,
    });
    const wrapper = requireEvent(events, "midi_analysis_job_status");
    const handle = new MidiAnalysisJobHandle(this.protocol, wrapper.status);
    if (options.onProgress) {
      handle.onProgress(options.onProgress);
    }
    return handle;
  }

  async startMidiProcess(
    options: MidiToolTaskOptions,
  ): Promise<MidiProcessJobHandle> {
    const events = await this.protocol.request({
      type: "start_process_midi_file",
      input: options.input,
      output: options.output,
      config: { tool: structuredClone(options.tool) },
    });
    const wrapper = requireEvent(events, "midi_process_status");
    if (wrapper.status.state === "idle") {
      throw new MeridianSubprocessError(
        "MIDI processing job did not enter a running state",
      );
    }
    const handle = new MidiProcessJobHandle(this.protocol, wrapper.status);
    if (options.onEvent) {
      handle.onEvent(options.onEvent);
    }
    return handle;
  }

  async mergeMidiFiles(
    options: MidiMergeOptions,
  ): Promise<MidiFilesMergedEvent> {
    const config: MidiFilesMergeConfig = {
      mode: options.config?.mode ?? "append_tracks",
      normalize_metadata_track: options.config?.normalize_metadata_track ??
        false,
      ppq_override: options.config?.ppq_override ?? null,
    };
    const events = await this.protocol.request({
      type: "merge_midi_files",
      inputs: [...options.inputs],
      output: options.output,
      config,
    });
    return requireEvent(events, "midi_files_merged");
  }

  async startAudioRender(
    options: AudioRenderOptions,
  ): Promise<AudioRenderJobHandle> {
    await this.resources.loadAudioMidi(options.midiPath);

    const config: SdkAudioRenderConfig = {
      midi_path: options.midiPath,
      output: options.output,
      sample_rate: options.sampleRate ?? null,
      channels: options.channels ?? null,
      use_limiter: options.useLimiter ?? null,
      format: options.format ?? "wav",
      ffmpeg_args: options.ffmpegArgs ?? [],
      soundfonts: options.soundfonts ?? [],
    };
    const events = await this.protocol.request({
      type: "start_render_audio",
      config,
    });
    const wrapper = requireEvent(events, "audio_render_status");
    if (wrapper.status.state === "idle") {
      throw new MeridianSubprocessError(
        "Audio render job did not enter a running state",
      );
    }
    const handle = new AudioRenderJobHandle(this.protocol, wrapper.status);
    if (options.onEvent) {
      handle.onEvent(options.onEvent);
    }
    return handle;
  }

  async startVideoRender(
    options: VideoRenderOptions,
  ): Promise<VideoRenderJobHandle> {
    const renderer = options.renderer ?? inferRendererFromScene(options.scene);
    if (!renderer && !options.scene) {
      throw new MeridianSubprocessError(
        "Video render requires either a renderer or a scene config",
      );
    }
    const config: ProtocolVideoRenderConfig = {
      midi_path: options.midiPath,
      output: options.output,
      container: options.container ?? "mp4",
      fps: options.fps,
      width: options.width,
      height: options.height,
      renderer,
      scene: options.scene ? structuredClone(options.scene) : null,
      view_range: options.viewRange ?? null,
      time_space: options.timeSpace ?? null,
      first_key: options.firstKey ?? null,
      last_key: options.lastKey ?? null,
      export: {
        color_mode: options.rgbMode ?? "premultiplied",
        export_alpha_mask: options.exportAlphaMask ?? false,
      },
      ffmpeg_args: options.ffmpegArgs ?? [],
      audio: null,
    };
    const events = await this.protocol.request({
      type: "start_render_video",
      config,
    });
    const wrapper = requireEvent(events, "video_render_status");
    if (wrapper.status.state === "idle") {
      throw new MeridianSubprocessError(
        "Video render job did not enter a running state",
      );
    }
    const handle = new VideoRenderJobHandle(this.protocol, wrapper.status);
    if (options.onEvent) {
      handle.onEvent(options.onEvent);
    }
    return handle;
  }

  private async loadParsedMidi(path: string): Promise<ParsedMidiId> {
    const events = await this.protocol.request({
      type: "load_parsed_midi",
      path,
    });
    const event = requireEvent(events, "parsed_midi_loaded");
    return event.parsed_midi_id;
  }

  private async loadAudioMidi(path: string): Promise<MidiLoadedEvent> {
    const events = await this.protocol.request({
      type: "load_audio_midi",
      path,
    });
    return requireEvent(events, "midi_loaded");
  }
}
