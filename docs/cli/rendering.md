# Rendering

The examples below assume your shell already has the required tools available.
On Linux, `nix-shell` is the easiest way to get that environment.

This page covers the CLI rendering commands: `render audio`, `render video`,
and the stdout-oriented `render frame`.

## Render Audio

Use `render audio` to turn a MIDI file into WAV, FLAC, or MP3.

```bash
cargo run -p meridian-cli -- render audio song.mid --output out.flac --format flac
```

Common knobs:

- `--format wav|flac|mp3`: choose the audio container
- `--sample-rate`: set the sample rate
- `--channels`: set the channel count
- `--no-limiter`: disable the default limiter
- `--soundfont`: pass one or more explicit soundfont paths

Example with an explicit soundfont:

```bash
cargo run -p meridian-cli -- render audio song.mid --output out.mp3 --format mp3 --sample-rate 44100 --channels 2 --soundfont /path/to/font.sf2
```

If you omit `--soundfont`, Meridian uses its embedded/default soundfont path
when available.

## Render Video

Use `render video` to write an encoded video file.

```bash
cargo run -p meridian-cli -- render video song.mid --output clip.mp4 --fps 30
```

The most useful knobs are:

- `--start-time` and `--end-time`: trim the rendered time range in seconds
- `--container mp4|mkv`: choose the output container
- `--fps`: set the frame rate
- `--width` and `--height`: set the output size
- `--view-range`: control the viewport time window around the playhead
- `--time-space time|tick`: choose the on-screen time scale
- `--renderer flat|pfa|text|piano-trail-classic`: choose the scene renderer
- `--rgb-mode premultiplied|straight`: choose the frame color mode
- `--export-alpha-mask`: include an alpha mask export when supported
- `--ffmpeg-flags`: pass extra `ffmpeg` flags as a single quoted string

Example with a custom range and explicit renderer:

```bash
cargo run -p meridian-cli -- render video song.mid --output clip.mkv --start-time -1.0 --end-time 18.0 --fps 60 --container mkv --renderer piano-trail-classic --ffmpeg-flags "-y"
```

Negative start times are accepted, which is useful when you want preroll before
the first visible event.

## Render Frame

Use `render frame` when you want a single rendered frame on stdout instead of a
file. That is useful for inspection, scripting, or shell-based comparisons.

```bash
cargo run -p meridian-cli -- render frame song.mid --time 12.5 --width 1280 --height 720 --renderer pfa
```

It shares the same renderer, time-space, and viewport controls as the video
workflow, but the output is a single frame rather than a sequence of encoded
video packets.

## Notes

- Encoded video and encoded audio workflows rely on `ffmpeg` being available.
  On Linux, the repo `nix-shell` provides it.
- Keep `--ffmpeg-flags` quoted as one string so the shell parser sees it as one
  option value.
- Match `--format` to the filename extension for audio output. The CLI rejects
  mismatches.
- `render video` is the right choice for exported clips; `render frame` is the
  right choice for a single image from the same scene pipeline.
