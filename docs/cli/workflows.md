# MIDI Workflows

This page covers the human-facing MIDI commands: `analyze`, `inspect`,
`process`, and `merge`.

## Analyze

Use `analyze` when you want a quick read on a single MIDI file.

```bash
nix-shell --run 'cargo run -p meridian-cli -- analyze song.mid --pretty'
```

The default analysis includes file, summary, events, notes, and tempo data.
You can narrow the report by repeating `--include`:

```bash
nix-shell --run 'cargo run -p meridian-cli -- analyze song.mid --include summary --include notes --pretty'
```

Useful knobs:

- `--include file|summary|events|notes|tempo`: choose which analysis groups to
  include
- `--buckets N`: add bucketed analysis output
- `--pretty`: format the JSON output for humans

If you want a broad overview, omit `--include`. If you want to script against a
small subset, pass only the kinds you need.

## Inspect

Use `inspect` when you want to inspect one or more MIDI files without changing
them.

```bash
nix-shell --run 'cargo run -p meridian-cli -- inspect song.mid other.mid --pretty'
```

That command is useful when you want a quick comparison across files or when
you want to confirm that a set of inputs load cleanly before a later workflow.

## Process

`process` is the curated MIDI transformation command. It exposes four
subcommands:

- `select`
- `tempo-flatten`
- `tempo-scale`
- `quantize`

### `process select`

Use `select` to cut out a tick range and optionally preserve leading system
events.

```bash
nix-shell --run 'cargo run -p meridian-cli -- process select song.mid --output excerpt.mid --start-ticks 0 --end-ticks 1920 --pretty'
```

Important knobs:

- `--start-ticks` and `--end-ticks` define the selected tick window
- `--offset-ticks` shifts the selected content to a new output origin
- `--track-select` limits the rewrite to a single track
- `--preserve-system-events` keeps supported system events before the range
- `--edge-behavior keep|skip|trim` controls how partial notes are handled

The default edge behavior is `trim`.

### `process tempo-flatten`

Use `tempo-flatten` to rewrite a file to a single tempo value.

```bash
nix-shell --run 'cargo run -p meridian-cli -- process tempo-flatten song.mid --output flat.mid --tempo 500000 --pretty'
```

The `--tempo` value is a MIDI tempo in microseconds per quarter note.

### `process tempo-scale`

Use `tempo-scale` to speed up or slow down the tempo map by a factor.

```bash
nix-shell --run 'cargo run -p meridian-cli -- process tempo-scale song.mid --output faster.mid --factor 2.0 --pretty'
```

Values greater than `1.0` speed up the file. Values between `0.0` and `1.0`
slow it down.

### `process quantize`

Use `quantize` to snap events to a tick grid.

```bash
nix-shell --run 'cargo run -p meridian-cli -- process quantize song.mid --output quantized.mid --grid-ticks 120 --mode note-start-and-end --pretty'
```

Useful knobs:

- `--grid-ticks`: the snapping grid in ticks
- `--mode note-start-only|note-start-and-end|all-events`: choose what gets
  quantized

The default mode is `note-start-only`.

## Merge

Use `merge` when you want to combine multiple MIDI files into one output.

```bash
nix-shell --run 'cargo run -p meridian-cli -- merge left.mid right.mid --output merged.mid --pretty'
```

`merge` accepts one or more input files and always requires `--output`.
`--pretty` only changes the JSON status output on stdout.

## Practical Notes

- `analyze`, `inspect`, `process`, and `merge` all emit machine-readable status
  JSON on stdout.
- `--pretty` is for readable CLI output, not for the MIDI files themselves.
- If you need a more programmable integration path, move up one level to the
  TypeScript SDK or use `stdio` directly.
