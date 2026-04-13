# UI Guide

Meridian is primarily a desktop UI application. Start here if you want to use
the program rather than script it.

## First Steps

1. [Preview](./preview.md): load a MIDI, play it, scrub around, and inspect the
   viewport
2. [Video](./video.md): choose the scene, framing, palette, and renderer
3. [Audio](./audio.md): tune the live synth and soundfont setup
4. [Render](./render.md): export video, audio, or both

## Editing Workflows

- [Modify](./modify.md): transform one MIDI file and save the result
- [Merge](./merge.md): combine multiple MIDI files into one output
- [Analysis](./analysis.md): inspect metrics without changing the file

## Notes

- The UI starts preview playback at `-1 s` so you get visible preroll before
  time zero.
- Loading a new MIDI resets the render range defaults to `-1 .. midi_length`.
- Scene and audio setup are remembered between launches. The selected MIDI file,
  current playhead position, output paths, and job status are not.
