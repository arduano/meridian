# Troubleshooting

Use this page when Meridian fails to start, rejects a request, or stops short
of a render. Start with the first visible error, then jump to the matching
section below.

If you are looking for normal workflows instead of failure recovery, use:

- [Repository README](../README.md)
- [Docs index](./README.md)
- [TypeScript SDK](./typescript-sdk/README.md)
- [Testing Guide](./testing.md)
- [Durable UI config](./ui-durable-config.md)

## Quick Triage

When something fails, check these first:

1. Are you running in the same shell that has the expected tools on `PATH`?
2. Does the selected output path match the chosen format or container?
3. Does the host have the runtime prerequisite for the path you are testing?
4. Is Meridian rejecting a bad request up front, or failing later while running?

The rest of this page is grouped in that order: setup issues first, then
validation failures, then lower-level runtime and recovery notes.

## `ffmpeg` Is Missing

Video rendering depends on `ffmpeg` being available on the host. The failure
usually shows up as one of these:

- `failed to spawn ffmpeg: ...`
- `ffmpeg exited with status ...`
- SDK or CLI video smoke tests skipping because `ffmpeg` is unavailable

What to do:

- install `ffmpeg`
- verify `ffmpeg -version` works in the same shell you use for Meridian
- if you are using `nix-shell`, stay in that shell for both build and run

Where this matters:

- [Video Rendering](./typescript-sdk/video-rendering.md)
- [Testing Guide](./testing.md)

## A Soundfont Is Missing

Audio rendering needs at least one usable soundfont. The most common failure
strings are:

- `soundfont resolve failed: ...`
- `soundfont load failed: ...`
- SDK or CLI audio smoke tests skipping because no soundfont is available

What to do:

- set `MERIDIAN_SOUNDFONT` to a readable `.sf2` path
- or point the UI/audio config at a real soundfont file
- or use the vendored soundfont under `assets/soundfonts` when the docs or
  tests are using repository-local fixtures

If you are writing SDK code, pass `soundfonts` explicitly when reproducibility
matters. The audio render guide documents the request shape.

Where this matters:

- [Audio Rendering](./typescript-sdk/audio-rendering.md)
- [TypeScript SDK README](./typescript-sdk/README.md)
- [Testing Guide](./testing.md)

## No Graphics Adapter / Headless WGPU

Some paths need a working graphics session:

- UI frame capture
- SDK and CLI save-frame smoke paths
- headless render probes that still require WGPU

The common failure text is:

- `request_adapter failed`
- `No suitable graphics adapter found`

What to do:

- run from a desktop session with a usable graphics adapter
- on Linux, make sure `WAYLAND_DISPLAY` or `DISPLAY` is set when you expect a
  normal interactive session
- if you only need non-graphics workflows, use the CLI/stdio audio or MIDI
  paths instead of frame capture

This is a host limitation, not a Meridian config problem.

Where this matters:

- [Reference: environment notes](./reference/environment.md)
- [Testing Guide](./testing.md)

## UI Config Version Mismatch

The durable UI config is versioned. If the saved file is from an older build,
Meridian now rejects it with a message like:

- `unsupported meridian-ui config version ... delete the saved UI config to regenerate defaults`

What to do:

- delete the saved UI config directory or the `config.json` file inside it
- restart Meridian so it regenerates defaults
- if you want the full storage layout and recovery behavior, read the durable
  config doc first

Meridian also tries to fall back to `config.json.bak` when the primary file is
invalid, so you may see a repair message instead of a hard failure.

Where this matters:

- [Durable UI config](./ui-durable-config.md)
- [Reference: environment notes](./reference/environment.md)

## Bad Output Extension

Meridian validates output paths before it starts a render job.

Audio output paths must match the selected format:

- `wav` -> `.wav`
- `flac` -> `.flac`
- `mp3` -> `.mp3`

Video output paths must match the selected container:

- `mp4` -> `.mp4`
- `mkv` -> `.mkv`

If the extension does not match, the request is rejected before the render
starts. In the UI this usually surfaces as an input error; in CLI/SDK code it
shows up as a `platform error` or protocol validation failure.

Where this matters:

- [Audio Rendering](./typescript-sdk/audio-rendering.md)
- [Video Rendering](./typescript-sdk/video-rendering.md)

## Common Validation Failures

These are the most common request-shape rejections. They are usually good
errors, not bugs:

- video `fps` must be greater than `0`
- video `width` and `height` must be greater than `0`
- video `start_time` and `end_time` must be finite values
- video `end_time` must be greater than `start_time`
- video `start_time` and `end_time` must not exceed the MIDI length
- viewport dimensions must be greater than zero
- `first_key` must be less than or equal to `last_key`
- modify and merge output paths must differ from their selected input paths
- the modify panel still needs a valid output path before a job can start

These checks happen early so Meridian can reject the request before it mutates
state or starts a render.

Where this matters:

- [Starting Points](./starting-points.md)
- [Testing Guide](./testing.md)

## Advanced Reference

The environment notes page collects the knobs that usually matter when a
troubleshooting session turns into a host-specific diagnosis:

- `MERIDIAN_SOUNDFONT`
- `MERIDIAN_TEST_SOUNDFONT`
- `MERIDIAN_UI_CONFIG_DIR`
- `MERIDIAN_FORCE_WGPU_TESTS`

See the reference page for the exact scope and caveats.

- [Reference: environment notes](./reference/environment.md)

## If None Of These Match

When the error text does not line up with any section above:

- search the repo for the exact message
- check the matching subsystem guide instead of guessing
- use the docs links above to get back to the workflow or test surface

That is usually faster than trying to reverse-engineer the whole stack from a
single failure message.
