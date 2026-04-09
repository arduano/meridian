# MIDI Modification Tools

This page documents every current MIDI modifier tool exposed through the
TypeScript SDK. The snippets use the high-level `client.modification.*` helpers.
Each section also links to a runnable example file in
[`sdk/typescript/examples`](../../sdk/typescript/examples).

Assume the snippets already have:

```ts
const client = await createDenoMeridianClient("./target/debug/meridian-cli");
const input = "input.mid";
const output = "output.mid";
```

## `range_select`

Example file:
- [`modification_range_select.ts`](../../sdk/typescript/examples/modification_range_select.ts)

Fields:
- `start_ticks` and `end_ticks` define the selected time span.
- `offset_ticks` optionally shifts the selected output to a new starting tick.
- `track_select` limits the operation to one track when set.
- `preserve_system_events` keeps selected pre-range system/meta events at the
  output start.
- `edge_behavior` controls whether overlapping notes are kept, skipped, or
  trimmed.

```ts
const result = await client.modification.rangeSelect({
  input,
  output,
  start_ticks: 0,
  end_ticks: 72,
  offset_ticks: 0,
  edge_behavior: "trim",
});
```

Limitations:
- `start_ticks` must be strictly less than `end_ticks`.
- `track_select`, when set, must point at an existing track.
- `preserve_system_events` only applies to a fixed set of pre-range
  system/meta events and does not preserve ordinary note/controller data from
  before the selected range.

## `tempo_map`

Example file:
- [`modification_tempo_map.ts`](../../sdk/typescript/examples/modification_tempo_map.ts)

Modes:
- `flatten(tempo)` removes existing tempo events and injects one tempo event.
- `scaleBpm(factor)` scales existing tempo events in place.
- `replace(points, destination)` replaces the tempo map with explicit points,
  either in track 0 or a new dedicated tempo track.

```ts
const result = await client.modification.tempoMap.flatten(600_000, {
  input,
  output,
});
```

Limitations:
- Tempo values must be in `1..=0xFF_FFFF`.
- `scale_bpm` requires a finite factor. During application, extremely small
  factors are clamped internally to avoid division by zero.
- `replace` sorts points by tick. An empty `points` list removes all tempo
  events.

## `time_warp`

Example file:
- [`modification_time_warp.ts`](../../sdk/typescript/examples/modification_time_warp.ts)

Fields:
- `points` is a list of `{ source_tick, dest_tick }` warp anchors.

```ts
const result = await client.modification.timeWarp({
  input,
  output,
  points: [
    { source_tick: 0, dest_tick: 0 },
    { source_tick: 96, dest_tick: 144 },
  ],
});
```

Limitations:
- Source ticks must be unique after sorting.
- Destination ticks must be monotonic so events cannot be reordered.
- Fewer than two points is effectively a no-op.
- Ticks after the last point snap to the last destination tick instead of being
  extrapolated.

## `channel_remap`

Example file:
- [`modification_channel_remap.ts`](../../sdk/typescript/examples/modification_channel_remap.ts)

Fields:
- `mappings` is a list of `{ from, to }` channel rewrites.

```ts
const result = await client.modification.channelRemap({
  input,
  output,
  mappings: [{ from: 0, to: 2 }],
});
```

Limitations:
- Channels must stay in `0..15`.
- Each source channel may only be mapped once.
- Unspecified channels are left unchanged.

## `track_route`

Example file:
- [`modification_track_route.ts`](../../sdk/typescript/examples/modification_track_route.ts)

Modes:
- `collapseAll()` merges every source track into one output track.
- `splitByChannel()` separates channelized events into per-channel tracks.
- `map(mappings)` rewrites source track indexes to target track indexes.

```ts
const result = await client.modification.trackRoute.splitByChannel({
  input,
  output,
});
```

Limitations:
- `map` validates source track indexes and requires each source to appear at
  most once.
- `map` allows sparse target indexes; empty target slots are skipped rather than
  written as blank tracks.
- `split_by_channel` creates up to 17 buckets: one for non-channel events and
  one for each MIDI channel.

## `program`

Example file:
- [`modification_program.ts`](../../sdk/typescript/examples/modification_program.ts)

Fields:
- `force_program` rewrites existing program-change events.
- `strip_program_changes` removes existing program-change events.
- `startup_programs` injects program changes at tick 0 in the first output
  track.

```ts
const result = await client.modification.program({
  input,
  output,
  force_program: 0,
  startup_programs: [{ channel: 0, program: 0 }],
});
```

Limitations:
- Channels must stay in `0..15`; program values must stay in `0..127`.
- Each startup channel may only appear once.
- `force_program` only rewrites existing program-change events. It does not add
  missing ones by itself.
- If `strip_program_changes` is true, existing program changes are removed even
  when `force_program` is also set.

## `control_change`

Example file:
- [`modification_control_change.ts`](../../sdk/typescript/examples/modification_control_change.ts)

Fields:
- `strip_controllers` removes matching controller numbers.
- `remap_controllers` rewrites controller numbers.
- `scale_controllers` rescales controller values.
- `inject_start` injects controller values at tick 0.

```ts
const result = await client.modification.controlChange({
  input,
  output,
  strip_controllers: [64],
  remap_controllers: [{ from: 1, to: 11 }],
  scale_controllers: [{ controller: 11, scale: 0.75 }],
  inject_start: [{ channel: 0, controller: 11, value: 100 }],
});
```

Limitations:
- Channels must stay in `0..15`; controller numbers and values must stay in
  `0..127`.
- Each controller may only be remapped once and scaled once.
- Each `(channel, controller)` pair may only appear once in `inject_start`.
- Strip checks happen after remapping, so a remapped controller can then be
  stripped.

## `pitch_bend`

Example file:
- [`modification_pitch_bend.ts`](../../sdk/typescript/examples/modification_pitch_bend.ts)

Fields:
- `strip` removes pitch-bend events entirely.
- `scale` and `offset` transform pitch-bend values.
- `min_bend` and `max_bend` clamp the final value.

```ts
const result = await client.modification.pitchBend({
  input,
  output,
  scale: 0.5,
  offset: 256,
  min_bend: -2048,
  max_bend: 2048,
});
```

Limitations:
- `scale` must be finite.
- `min_bend` and `max_bend` must stay within `-8192..8191`.
- `min_bend` must be less than or equal to `max_bend`.
- The tool only affects existing pitch-bend events; it does not synthesize new
  ones.

## `velocity_map`

Example file:
- [`modification_velocity_map.ts`](../../sdk/typescript/examples/modification_velocity_map.ts)

Modes:
- `scale(scale)` multiplies velocities.
- `gamma(gamma)` applies a gamma curve.
- `polyline(points)` interpolates between explicit velocity points.

```ts
const result = await client.modification.velocityMap.polyline([
  { input: 0, output: 0 },
  { input: 64, output: 96 },
  { input: 127, output: 127 },
], {
  input,
  output,
});
```

Limitations:
- `scale` and `gamma` must be finite.
- `polyline` cannot contain duplicate input velocities.
- Output values are clamped to `0..127`.
- The tool updates note-on and polyphonic key-pressure velocities. It does not
  rewrite unrelated controller or note timing data.

## `change_ppq`

Example file:
- [`modification_change_ppq.ts`](../../sdk/typescript/examples/modification_change_ppq.ts)

Fields:
- `ppq` is the new pulses-per-quarter-note resolution.

```ts
const result = await client.modification.changePpq(192, {
  input,
  output,
});
```

Limitations:
- `ppq` must be greater than zero.
- Event deltas are rescaled to the new PPQ; track count and event ordering stay
  otherwise unchanged.

## `extract_track`

Example file:
- [`modification_extract_track.ts`](../../sdk/typescript/examples/modification_extract_track.ts)

Fields:
- `track_index` selects the source track to copy into the output file.

```ts
const result = await client.modification.extractTrack(0, {
  input,
  output,
});
```

Limitations:
- `track_index` must refer to an existing source track.
- The output always contains exactly one track.

## `note_length`

Example file:
- [`modification_note_length.ts`](../../sdk/typescript/examples/modification_note_length.ts)

Fields:
- `scale` rescales note lengths.
- `fixed_ticks` overwrites note lengths with one fixed value.
- `min_ticks` and `max_ticks` clamp the final result.

```ts
const result = await client.modification.noteLength({
  input,
  output,
  scale: 1.5,
  min_ticks: 24,
});
```

Limitations:
- `scale` must be finite and non-negative when set.
- `fixed_ticks` is applied before `min_ticks` and `max_ticks`.
- Notes are always written with a minimum length of one tick.

## `quantize`

Example file:
- [`modification_quantize.ts`](../../sdk/typescript/examples/modification_quantize.ts)

Fields:
- `rounding_ticks` is the quantization grid size.
- `mode` chooses between `note_start_only`, `note_start_and_end`, and
  `all_events`.

```ts
const result = await client.modification.quantize({
  input,
  output,
  rounding_ticks: 24,
  mode: "note_start_and_end",
});
```

Limitations:
- `rounding_ticks` must be greater than zero.
- `note_start_only` leaves note ends and non-note events untouched.
- `note_start_and_end` guarantees a minimum output note length of one tick.
- `all_events` quantizes every event stream timestamp, including tempo/meta
  events.

## `humanize`

Example file:
- [`modification_humanize.ts`](../../sdk/typescript/examples/modification_humanize.ts)

Fields:
- `start_jitter`, `length_jitter`, and `velocity_jitter` define the maximum
  symmetric jitter range.
- `seed` makes the output deterministic.
- `collision_mode` controls whether otherwise-identical notes share jitter.

```ts
const result = await client.modification.humanize({
  input,
  output,
  start_jitter: 4,
  length_jitter: 8,
  velocity_jitter: 12,
  seed: 7,
  collision_mode: "distinguish_by_note",
});
```

Limitations:
- Jitter magnitudes are treated symmetrically around zero; negative inputs are
  effectively handled by absolute value.
- Note starts are clamped so they cannot move earlier than the previous emitted
  note start in the same track.
- Output note lengths are clamped to at least one tick and velocities to
  `1..127`.

## `key_map`

Example file:
- [`modification_key_map.ts`](../../sdk/typescript/examples/modification_key_map.ts)

Fields:
- `mappings` rewrites explicit keys.
- `fold_to_range` folds otherwise-unmapped keys into a range.
- `drop_unmapped` drops unmapped keyed events when folding is not used.

```ts
const result = await client.modification.keyMap({
  input,
  output,
  mappings: [{ from: 60, to: 67 }],
  fold_to_range: { min: 48, max: 84 },
});
```

Limitations:
- Explicit mappings override the default fold/drop behavior.
- `fold_to_range` normalizes reversed ranges automatically.
- The tool rewrites any keyed MIDI event, not just note-on/note-off pairs.

## `meta_text`

Example file:
- [`modification_meta_text.ts`](../../sdk/typescript/examples/modification_meta_text.ts)

Fields:
- `keep_kinds` is the allow-list of text meta-event kinds to keep.

```ts
const result = await client.modification.metaText({
  input,
  output,
  keep_kinds: ["track_name", "marker"],
});
```

Limitations:
- An empty `keep_kinds` list keeps all text meta events.
- This tool only filters `Text` meta events. It does not rewrite the text bytes
  and it does not touch unknown meta events.

## `sysex`

Example file:
- [`modification_sysex.ts`](../../sdk/typescript/examples/modification_sysex.ts)

Fields:
- `strip_all` removes existing sysex and end-of-exclusive events.
- `prepend` injects raw sysex payloads at tick 0.

```ts
const result = await client.modification.sysex({
  input,
  output,
  strip_all: true,
  prepend: [[0x7d, 0x01, 0x02, 0x03]],
});
```

Limitations:
- Prepended messages are inserted into the first output track, or into a new
  track when the input has none.
- `prepend` forwards raw payload bytes as provided; the SDK does not validate or
  interpret manufacturer/message structure for you.

## `shared_metadata_track`

Example file:
- [`modification_shared_metadata_track.ts`](../../sdk/typescript/examples/modification_shared_metadata_track.ts)

Fields:
- `destination` is either `{ mode: "create_new" }` or
  `{ mode: "insert_into", track_index }`.
- `strip_redundant_events` removes redundant shared state events while merging.
- `move_*` flags choose which classes are extracted: tempo, time signatures,
  key signatures, text, unknown meta, channel prefix, MIDI port, control
  change, program change, pitch bend, and channel pressure.

```ts
const result = await client.modification.sharedMetadataTrack({
  input,
  output,
  destination: { mode: "create_new" },
  strip_redundant_events: true,
  move_tempo_events: true,
  move_time_signatures: true,
  move_key_signatures: true,
});
```

Limitations:
- `insert_into.track_index` must refer to an existing track.
- If no `move_*` flags are enabled, or no matching events exist, the file is
  written back without creating a new metadata track.
- Redundancy stripping only deduplicates stateful shared-event classes. Text
  and unknown-meta events are not collapsed.
- With `create_new`, the shared metadata track becomes the first output track.
