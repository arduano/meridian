# Modify

Use `Modify` to write a transformed MIDI file.

## Basic Workflow

1. Load a MIDI for analysis.
2. Choose a pass.
3. Tune the controls.
4. Pick an output path.
5. Run the job.

## Common Controls

- pass selector: chooses the transformation
- structured controls: the normal UI for the selected pass
- JSON editor: direct editing for complex configurations
- output path: destination for the modified MIDI

## Per-Knob Guidance

- output path
  It must not overwrite the selected input file.
- structured controls vs JSON
  The structured controls are the easiest path. Use the JSON editor when the
  pass has more detail than the panel exposes at once.
- `Tempo Map` destination
  `First Track` rewrites the existing leading tempo track. `New Tempo Track`
  creates a separate tempo track instead.
- `Time Warp`
  Uses `source_tick -> dest_tick` pairs. Use it when simple scaling is not
  enough.
- `Range Select`
  Limits later passes to specific tracks, channels, keys, ticks, velocities, or
  event families.

## Typical Passes

- `Quantize`
- `Tempo Map`
- `Time Warp`
- `Channel Remap`
- `Track Route`
- `Program`
- `Control Change`
- `Pitch Bend`
- `Velocity Map`
- `Note Length`
- `Humanize`
- `Key Map`
- `Meta Text`
- `Sysex`
- `Change PPQ`
- `Extract Track`
- `Shared Metadata Track`

## What To Expect

- invalid JSON is rejected without destroying the last valid config
- the output path must be valid before the job starts
- modify defaults are remembered, but output paths are not
