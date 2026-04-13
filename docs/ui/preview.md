# Preview

Use `Preview` to load a MIDI, play it back, and inspect the viewport before you
change any rendering or export settings.

## Basic Workflow

1. Click `Load for Preview`.
2. Press `Play`.
3. Drag the time scrubber to move around the song.
4. Use the viewport to inspect what is currently visible.

The transport starts at `-1 s`, not `0 s`, so there is preroll before the
first event.

## Main Controls

- `Load for Preview`: loads the current MIDI for playback and viewport preview
- `Play` / `Pause`: starts or stops transport playback
- time scrubber: seeks through the loaded file and accepts negative preroll
- `Time Space`: changes whether the viewport scale is shown in seconds or ticks
- `View Range`: zooms the visible window around the playhead

## What To Expect

- the title bar shows the currently loaded MIDI
- the playhead advances while playing
- the viewport follows the current play position
- the preview file itself is not remembered after restart
