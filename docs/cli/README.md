# CLI Guide

This guide is for the curated Meridian CLI surface in
[`crates/meridian-cli`](../../crates/meridian-cli). The CLI is intentionally
small and direct: use it for local work, use `stdio` for custom automation, and
use the TypeScript SDK when you want a higher-level programmable integration.

## Start Here

- [Workflows](./workflows.md): analyze, inspect, process, and merge MIDI files
- [Rendering](./rendering.md): audio, video, and frame export examples
- [stdio Mode](./stdio.md): long-lived protocol mode and the raw `json` helper
- [Testing Guide](../testing.md): repo-local validation commands

## First Commands

These are the shortest useful commands to try first:

```bash
nix-shell --run 'cargo run -p meridian-cli -- --help'
nix-shell --run 'cargo run -p meridian-cli -- analyze song.mid --pretty'
nix-shell --run 'cargo run -p meridian-cli -- inspect song.mid --pretty'
nix-shell --run 'cargo run -p meridian-cli -- stdio'
```

If you only want to confirm the command surface, `--help` is the first check.
If you want to confirm that a MIDI file parses, `analyze` is the fastest
end-to-end command. If you want to drive Meridian from another process, start
with `stdio`.

## Curated Posture

The CLI does not expose every internal protocol feature as a separate command.
That is deliberate.

- `analyze`, `inspect`, `process`, `merge`, and `render` cover the common local
  workflows.
- `stdio` is the stable automation surface for custom frontends and scripts.
- `json` is the raw one-shot protocol helper when you want to send a single
  request without keeping a process open.
- The TypeScript SDK is the preferred place for larger integrations that need a
  richer client API.

## Command Map

- `analyze`: inspect a single MIDI file and emit analysis JSON
- `inspect`: inspect one or more MIDI files
- `process`: modify MIDI files with curated subcommands
- `merge`: combine MIDI files into a single output
- `render audio`: render a MIDI file to WAV, FLAC, or MP3
- `render video`: render a MIDI file to MP4 or MKV
- `render frame`: write a single rendered frame to stdout
- `stdio`: run the line-delimited JSON protocol server
- `json`: send one raw protocol request and exit

## Reading Order

Read the linked pages in this order if you are new to the CLI:

1. [Workflows](./workflows.md)
2. [Rendering](./rendering.md)
3. [stdio Mode](./stdio.md)

That progression goes from the smallest successful commands to the more
configurable paths and finally to the automation surface.
