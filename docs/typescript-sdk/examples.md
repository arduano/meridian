# Examples

The examples live in
[`sdk/typescript/examples`](../../sdk/typescript/examples). They are small,
runnable Deno scripts intended to be copied into local experiments or used as a
reference while reading the SDK source. Where possible they import through the
public SDK entrypoint rather than internal runtime files.

## Core Workflow Examples

- [`analysis.ts`](../../sdk/typescript/examples/analysis.ts): basic MIDI
  analysis flow
- [`resources.ts`](../../sdk/typescript/examples/resources.ts): explicit
  resource loading
- [`process.ts`](../../sdk/typescript/examples/process.ts): generic MIDI
  modification flow using `key_map`
- [`audio_render.ts`](../../sdk/typescript/examples/audio_render.ts): audio
  rendering flow
- [`video_render.ts`](../../sdk/typescript/examples/video_render.ts): video
  rendering flow with explicit `mkv` container selection
- [`video_render_scene.ts`](../../sdk/typescript/examples/video_render_scene.ts):
  video rendering with full scene-config passthrough
- [`raw_protocol.ts`](../../sdk/typescript/examples/raw_protocol.ts): direct
  protocol usage
- [`_shared.ts`](../../sdk/typescript/examples/_shared.ts): shared helper setup
  for local examples

## Modifier Tool Examples

- [`modification_range_select.ts`](../../sdk/typescript/examples/modification_range_select.ts)
- [`modification_tempo_map.ts`](../../sdk/typescript/examples/modification_tempo_map.ts)
- [`modification_time_warp.ts`](../../sdk/typescript/examples/modification_time_warp.ts)
- [`modification_channel_remap.ts`](../../sdk/typescript/examples/modification_channel_remap.ts)
- [`modification_track_route.ts`](../../sdk/typescript/examples/modification_track_route.ts)
- [`modification_program.ts`](../../sdk/typescript/examples/modification_program.ts)
- [`modification_control_change.ts`](../../sdk/typescript/examples/modification_control_change.ts)
- [`modification_pitch_bend.ts`](../../sdk/typescript/examples/modification_pitch_bend.ts)
- [`modification_velocity_map.ts`](../../sdk/typescript/examples/modification_velocity_map.ts)
- [`modification_change_ppq.ts`](../../sdk/typescript/examples/modification_change_ppq.ts)
- [`modification_extract_track.ts`](../../sdk/typescript/examples/modification_extract_track.ts)
- [`modification_note_length.ts`](../../sdk/typescript/examples/modification_note_length.ts)
- [`modification_quantize.ts`](../../sdk/typescript/examples/modification_quantize.ts)
- [`modification_humanize.ts`](../../sdk/typescript/examples/modification_humanize.ts)
- [`modification_key_map.ts`](../../sdk/typescript/examples/modification_key_map.ts)
- [`modification_meta_text.ts`](../../sdk/typescript/examples/modification_meta_text.ts)
- [`modification_sysex.ts`](../../sdk/typescript/examples/modification_sysex.ts)
- [`modification_shared_metadata_track.ts`](../../sdk/typescript/examples/modification_shared_metadata_track.ts)

Each modifier example uses the same repo MIDI fixture from `assets/midis/piano`
and prints the produced output path plus the resulting track count. That keeps
the examples easy to type-check and easy to diff against each other while still
showing a more realistic input file.

Related deep-dive pages:

- [MIDI Modification Tools](./midi-modification-tools.md)
- [Audio Rendering](./audio-rendering.md)
- [Video Rendering](./video-rendering.md)
