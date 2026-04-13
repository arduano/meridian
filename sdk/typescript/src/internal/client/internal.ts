import type {
  CoreEvent,
  ErrorEvent,
  MidiAnalysisKind,
  MidiModifierTool,
  ProtocolVideoRenderConfig,
} from "../../protocol.ts";

export class MeridianProtocolError extends Error {
  readonly code: ErrorEvent["code"];
  readonly events: CoreEvent[];

  constructor(event: ErrorEvent, events: CoreEvent[]) {
    super(event.message);
    this.name = "MeridianProtocolError";
    this.code = event.code;
    this.events = events;
  }
}

export class MeridianSubprocessError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "MeridianSubprocessError";
  }
}

export type MeridianEventListener = (event: CoreEvent) => void;

export function isErrorEvent(event: CoreEvent): event is ErrorEvent {
  return event.type === "error";
}

export function firstError(events: CoreEvent[]): ErrorEvent | null {
  return events.find(isErrorEvent) ?? null;
}

export function assertNoError(events: CoreEvent[]): void {
  const event = firstError(events);
  if (event) {
    throw new MeridianProtocolError(event, events);
  }
}

export function requireEvent<T extends CoreEvent["type"]>(
  events: CoreEvent[],
  type: T,
): Extract<CoreEvent, { type: T }> {
  assertNoError(events);
  const event = events.find(
    (candidate): candidate is Extract<CoreEvent, { type: T }> =>
      candidate.type === type,
  );
  if (!event) {
    throw new MeridianSubprocessError(
      `Expected event '${type}' but received ${
        events.map((item) => item.type).join(", ")
      }`,
    );
  }
  return event;
}

export function uniqueKinds(kinds: MidiAnalysisKind[]): MidiAnalysisKind[] {
  return [...new Set(kinds)];
}

export function inferRendererFromScene(
  scene: ProtocolVideoRenderConfig["scene"] | undefined,
): ProtocolVideoRenderConfig["renderer"] {
  if (!scene) {
    return null;
  }
  if (scene.scene_type === "three_d") {
    return "piano_trail_classic";
  }
  if (scene.scene_type === "text") {
    return null;
  }
  return scene.notes.projector === "flat" ? "flat" : "pfa";
}

export type ToolConfig<T extends MidiModifierTool> = Omit<T, "tool">;
