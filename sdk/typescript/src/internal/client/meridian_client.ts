import { midiTools } from "../../helpers.ts";
import type {
  AudioRenderEvent,
  ChangePpqTool,
  ChannelRemapTool,
  ControlChangeTool,
  FrameSavedEvent,
  ExtractTrackTool,
  HumanizeTool,
  KeyMapTool,
  MetaTextTool,
  MidiFilesMergedEvent,
  MidiFilesInspectedEvent,
  MidiLoadedEvent,
  MidiModifierTool,
  MidiProcessEvent,
  NoteLengthTool,
  ParsedMidiId,
  ProtocolStateSnapshot,
  PitchBendTool,
  ProgramTool,
  QuantizeTool,
  RangeSelectTool,
  SharedMetadataTrackTool,
  SceneConfig,
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
  SaveFrameOptions,
  VideoRenderOptions,
} from "./options.ts";
import {
  AudioRenderJobHandle,
  MidiAnalysisJobHandle,
  MidiProcessJobHandle,
  VideoRenderJobHandle,
} from "./handles.ts";
import { type ToolConfig } from "./internal.ts";
import {
  normalizeMidiMergeConfig,
  normalizeSaveFrameOptions,
  toSdkAudioRenderConfig,
  toProtocolVideoRenderConfig,
} from "./normalizers.ts";
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
    loadMidi: (path: string): Promise<MidiLoadedEvent> => this.loadMidi(path),
    loadAudioMidi: (path: string): Promise<MidiLoadedEvent> =>
      this.loadAudioMidi(path),
    inspectMidiFiles: (paths: string[]): Promise<MidiFilesInspectedEvent["inspections"]> =>
      this.inspectMidiFiles(paths),
  };
  readonly display = {
    setTime: (time: number): Promise<ProtocolStateSnapshot> => this.setTime(time),
    setSceneConfig: (scene: SceneConfig): Promise<ProtocolStateSnapshot> =>
      this.setSceneConfig(scene),
    setViewRange: (
      seconds: number,
      timeSpace?: ProtocolStateSnapshot["time_space"] | null,
    ): Promise<ProtocolStateSnapshot> => this.setViewRange(seconds, timeSpace),
    setKeyRange: (
      firstKey: number,
      lastKey: number,
    ): Promise<ProtocolStateSnapshot> => this.setKeyRange(firstKey, lastKey),
    setViewport: (
      width: number,
      height: number,
    ): Promise<ProtocolStateSnapshot> => this.setViewport(width, height),
    saveFrame: (options: SaveFrameOptions): Promise<FrameSavedEvent> =>
      this.saveFrame(options),
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
      kinds: options.kinds,
      bucket_count: options.bucketCount,
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
    const config = normalizeMidiMergeConfig(options.config);
    const events = await this.protocol.request({
      type: "merge_midi_files",
      inputs: [...options.inputs],
      output: options.output,
      config,
    });
    return requireEvent(events, "midi_files_merged");
  }

  async inspectMidiFiles(
    paths: string[],
  ): Promise<MidiFilesInspectedEvent["inspections"]> {
    const events = await this.protocol.request({
      type: "inspect_midi_files",
      paths: [...paths],
    });
    return requireEvent(events, "midi_files_inspected").inspections;
  }

  async startAudioRender(
    options: AudioRenderOptions,
  ): Promise<AudioRenderJobHandle> {
    await this.resources.loadAudioMidi(options.midiPath);
    const events = await this.protocol.request({
      type: "start_render_audio",
      config: toSdkAudioRenderConfig(options),
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
    const events = await this.protocol.request({
      type: "start_render_video",
      config: toProtocolVideoRenderConfig(options),
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

  async setTime(time: number): Promise<ProtocolStateSnapshot> {
    const events = await this.protocol.request({
      type: "set_time",
      time,
    });
    return requireEvent(events, "state_snapshot").state;
  }

  async setSceneConfig(scene: SceneConfig): Promise<ProtocolStateSnapshot> {
    const events = await this.protocol.request({
      type: "set_scene_config",
      scene: structuredClone(scene),
    });
    return requireEvent(events, "state_snapshot").state;
  }

  async setViewRange(
    seconds: number,
    timeSpace?: ProtocolStateSnapshot["time_space"] | null,
  ): Promise<ProtocolStateSnapshot> {
    const events = await this.protocol.request({
      type: "set_view_range",
      seconds,
      time_space: timeSpace ?? null,
    });
    return requireEvent(events, "state_snapshot").state;
  }

  async setKeyRange(
    firstKey: number,
    lastKey: number,
  ): Promise<ProtocolStateSnapshot> {
    const events = await this.protocol.request({
      type: "set_key_range",
      first_key: firstKey,
      last_key: lastKey,
    });
    return requireEvent(events, "state_snapshot").state;
  }

  async setViewport(
    width: number,
    height: number,
  ): Promise<ProtocolStateSnapshot> {
    const events = await this.protocol.request({
      type: "set_viewport",
      width,
      height,
    });
    return requireEvent(events, "state_snapshot").state;
  }

  async saveFrame(
    options: SaveFrameOptions,
  ): Promise<FrameSavedEvent> {
    const normalized = normalizeSaveFrameOptions(options);
    const events = await this.protocol.request({
      type: "save_frame",
      output: normalized.output,
      format: normalized.format ?? null,
      viewport_width: normalized.viewportWidth ?? null,
      viewport_height: normalized.viewportHeight ?? null,
      export: normalized.export,
    });
    return requireEvent(events, "frame_saved");
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

  private async loadMidi(path: string): Promise<MidiLoadedEvent> {
    const events = await this.protocol.request({
      type: "load_midi",
      path,
    });
    return requireEvent(events, "midi_loaded");
  }
}
