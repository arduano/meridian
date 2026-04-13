# UI Guide

This guide follows the Meridian UI from first launch to the more advanced tabs.
Read it top to bottom once, then jump back to the section you need when you
forget where a setting lives.

## First Launch

The UI opens with a preview-oriented workflow:

- load a MIDI file
- play or pause it
- scrub the current time
- inspect the viewport

If you open the app with a MIDI already selected, the title bar shows that file
name immediately. The preview start time now begins at `-1 s`, so there is
pre-roll available before time zero.

The main tabs are:

- `Preview` for transport and viewport playback
- `Video` for scene authoring and display settings
- `Audio` for live synth configuration
- `Render` for export
- `Modify` for single-file MIDI processing
- `Merge` for combining multiple MIDI files
- `Analysis` for read-only metrics

## Preview Workflow

Start here if you just want to hear and inspect a MIDI file.

1. Use `Load for Preview` to load the file.
2. Press `Play` to start playback.
3. Use the scrubber to seek around the song.
4. Switch `Time Space` between `Time` and `Tick` if you want to read the
   viewport in seconds or ticks.
5. Adjust `View Range` if you want to zoom the viewport in or out.

Expected outcome:

- the transport moves the current time forward while playing
- the viewport follows the current playhead position
- the loaded MIDI name stays visible at the top
- the time scrubber accepts negative pre-roll and valid song time

## Video Tab

Use `Video` when you want to shape the visual scene rather than the playback
engine.

The `Load for Video` button builds the display cache and preview viewport for
scene authoring. The panel is for:

- scene type selection
- renderer-specific controls
- palette, background, and aura choices
- text-scene overlays and formatting
- viewport framing controls

The key framing controls are `Time Space`, `View Range`, and the first/last key
range. The scene summary at the bottom is a quick readout of the active render
shape.

Expected outcome:

- changes here affect how the preview and exported video look
- scene settings persist across restarts
- the selected MIDI file does not persist as durable state

## Audio Tab

Use `Audio` when you want to tune realtime synth playback.

`Load for Audio` builds the audio cache only. This page is not the export page;
it is for live playback configuration such as:

- sample rate
- channel count
- buffer size
- soundfont selection
- interpolation and effects
- voice and layer limits
- limiter and threading behavior
- envelope curve shaping

Expected outcome:

- playback uses the configured synth parameters
- audio configuration persists
- the current engine status does not persist

## Modify Tab

Use `Modify` when you want to write a transformed MIDI file.

The workflow is:

1. Load a MIDI for analysis.
2. Pick a pass from the pass library.
3. Tune the structured controls or edit the JSON directly.
4. Choose an output path.
5. Run the job and inspect the result summary.

The pass library starts with a preset such as `Quantize`, but the tab also
supports more advanced tools like `Tempo Map`, `Time Warp`, `Channel Remap`,
`Track Route`, `Program`, `Control Change`, `Pitch Bend`, `Velocity Map`,
`Note Length`, `Humanize`, `Key Map`, `Meta Text`, `Sysex`, `Change PPQ`,
`Extract Track`, and `Shared Metadata Track`.

For the more advanced knobs, keep these examples in mind:

- `Tempo Map` can scale BPM or replace tempo points.
- `Tempo Map` destination chooses between `First Track` and `New Tempo Track`.
- `Time Warp` uses point pairs in `source_tick -> dest_tick` form.
- `Range Select` narrows later passes to specific tracks, channels, keys, ticks,
  velocities, or event families.

Expected outcome:

- the JSON editor and structured controls stay in sync
- invalid JSON is rejected without destroying the last valid config
- the output path must not overwrite the selected input MIDI

## Merge Tab

Use `Merge` when you want to combine several MIDI files into one output.

The workflow is:

1. Add files with the picker or drag and drop.
2. Reorder or remove sources as needed.
3. Choose the merge layout and metadata behavior.
4. Set an output path.
5. Run the merge job.

The merge recipe is intentionally smaller than the modify surface:

- `append_tracks` or `merge_tracks` controls track layout
- metadata mode controls whether the metadata track stays as-is or is normalized
- PPQ override is available when you need a specific resolution

Expected outcome:

- the queue shows per-source inspection status
- merge output cannot overwrite any input file
- source order matters for the resulting file layout

## Render Tab

Use `Render` when you want to export audio or video.

The workflow is:

1. Choose `Video + Audio`, `Video Only`, or `Audio Only`.
2. Set an output path.
3. Pick a full-song or custom time range.
4. Verify the stream settings.
5. Start the export.

The export panel has three important layers:

- mode decides whether the job emits video, audio, or both
- range decides whether export covers the full song or a custom interval
- stream settings decide the codec, container, sample rate, bitrate, and
  related knobs

The main defaults to know are:

- full-song video starts at `-1 s`
- custom ranges accept arbitrary negative start times
- `Open after export` opens the finished output only after success

Expected outcome:

- `Video + Audio` writes a single container file
- `Video Only` writes silent video
- `Audio Only` writes an audio file in the selected format
- the inline status text switches between `Ready`, `Running`, `Finished`,
  `Cancelled`, and `Failed`

## Analysis Tab

Use `Analysis` for read-only metrics about the loaded MIDI file.

This tab is not part of the basic playback path. It is useful when you want to
check note counts, tempo statistics, key range, event composition, and file
metrics before you export or modify anything.

## Persistence

The UI persists the last-used setup for the parts that are meant to survive a
restart. The detailed list lives in [persistence.md](./persistence.md).

Short version:

- persistent: scene, audio, view range, key range, export defaults, modify
  defaults, merge defaults, window geometry, and remembered asset paths
- not persistent: selected MIDI file, current playhead time, job status, source
  queues, output paths, and temporary runtime caches

## UI-Specific Troubleshooting

Use this section only for UI behavior that is specific to Meridian.

- If the wrong tab is loaded after a restart, check the persisted profile and
  panel defaults first.
- If a render range looks wrong after loading a new MIDI, remember that the UI
  resets the custom range to `-1 .. midi_length` when a new MIDI is loaded.
- If modify or merge tries to reuse a stale output path, clear the field and
  choose a new destination.
