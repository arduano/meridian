# Audio

Use `Audio` to control live playback synthesis. This page affects what you hear
in the preview path, not just what gets exported later.

## Main Controls

- `Load for Audio`: builds the audio cache used for playback
- sample rate: sets the playback sample rate
- channel count: chooses mono, stereo, or another supported layout
- buffer size: trades latency against stability
- soundfont selection: chooses the synthesis input
- interpolation / effects / limiter: changes playback quality and dynamics
- voice and layer limits: controls synthesis load

## Per-Knob Guidance

- sample rate
  Higher sample rates can improve quality but cost more CPU.
- buffer size
  Smaller buffers reduce latency but can underrun on weaker systems.
- soundfont
  If playback is silent or errors, verify this points to a real `.sf2`.
- limiter
  Leave it enabled unless you specifically want unclamped peaks.
- voice / layer limits
  Lower them if playback becomes too heavy; raise them if dense passages drop
  notes.

## What Persists

Audio configuration is remembered between launches. The current engine status
and temporary caches are not.
