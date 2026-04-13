# Render

Use `Render` to export video, audio, or both.

## Basic Workflow

1. Choose `Video + Audio`, `Video Only`, or `Audio Only`.
2. Pick an output path.
3. Choose `Full Song` or a custom time range.
4. Adjust the stream settings.
5. Start the export.

## Main Controls

- mode: chooses whether the job writes video, audio, or both
- output path: destination file; must match the selected container or format
- full song / custom range: chooses whether export covers the whole file or a
  selected interval
- `Open after export`: opens the output only after a successful job

## Per-Knob Guidance

- `Video + Audio`
  Writes one muxed container file and finishes only when both parts are done.
- `Video Only`
  Writes silent video.
- `Audio Only`
  Writes only the selected audio format.
- full song
  Uses `-1 s` as the default start so exports include preroll.
- custom start / end
  Accept finite negative start times. `end` must be greater than `start`.
- video container
  Match the output extension to the selected container: `.mp4` or `.mkv`.
- audio format
  Match the output extension to the selected format: `.wav`, `.flac`, or
  `.mp3`.
- MP3 bitrate
  Only matters when the audio output is MP3.

## Status

The inline export status moves through:

- `Ready`
- `Running`
- `Finished`
- `Cancelled`
- `Failed`

## What Resets

When you load a new MIDI, the render range resets to `-1 .. midi_length` so an
old file's custom range does not leak into the new export.
