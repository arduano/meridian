# Video

Use `Video` to shape how the preview and exported video look.

## Main Controls

- `Load for Video`: builds the display cache used for scene authoring
- scene type / renderer: chooses the visual style
- `Time Space`: changes the horizontal scale between time and ticks
- `View Range`: changes how much of the song is visible around the playhead
- first / last key: limits the visible keyboard range
- palette / background / aura assets: changes the color and texture inputs

## Scene Setup

The most important decisions on this page are:

1. pick the renderer
2. frame the viewport with `View Range` and key range
3. adjust the scene-specific controls for that renderer

## Per-Knob Guidance

- `Time Space`
  Use `Time` when you want a fixed seconds-based view. Use `Tick` when you want
  the view to track musical resolution instead.
- `View Range`
  Smaller values zoom in around the current playhead. Larger values show more
  surrounding context.
- first / last key
  Narrow these when you want to focus on a subset of the keyboard instead of
  showing the full range.
- palette / background / aura
  These affect scene styling, not playback. If an asset path is bad, the UI may
  fall back or show an error depending on the asset type.

## What Persists

Scene choices, viewport framing, and remembered asset paths survive restart.
The selected MIDI file does not.
