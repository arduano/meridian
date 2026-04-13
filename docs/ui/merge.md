# Merge

Use `Merge` to combine multiple MIDI files into one result.

## Basic Workflow

1. Add input files.
2. Reorder or remove them as needed.
3. Choose the merge layout.
4. Pick an output path.
5. Run the merge.

## Main Controls

- source list: the current merge queue
- order controls: change the source order
- layout mode: chooses how the tracks are combined
- metadata mode: decides how metadata is preserved or normalized
- PPQ override: forces a chosen resolution on output

## Per-Knob Guidance

- source order
  It affects the resulting track layout.
- layout mode
  `append_tracks` keeps tracks separate. `merge_tracks` combines compatible
  tracks into a shared output layout.
- metadata mode
  Use normalization when you want a cleaner merged metadata track.
- output path
  It must not overwrite any selected input file.

## What To Expect

- the queue shows per-source inspection status
- stale background inspection results should not replace the current queue view
- merge defaults are remembered, but the queue and output path are not
