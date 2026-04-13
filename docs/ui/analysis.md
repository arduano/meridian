# Analysis

Use `Analysis` when you want read-only metrics about the currently loaded MIDI.

## What It Is For

This tab is useful when you want to inspect the file before exporting,
modifying, or merging anything.

## Typical Uses

- confirm note counts and density
- inspect tempo statistics
- inspect key range
- inspect event composition
- inspect file-level metrics

## What To Expect

- analysis does not modify the loaded MIDI
- it is a reference panel, not part of the normal playback path
- if you want a scripting-first version of the same information, use the CLI or
  TypeScript SDK analysis paths
