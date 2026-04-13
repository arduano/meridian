# UI Persistence

This page explains what the Meridian UI remembers across launches and what it
resets on purpose.

## Persisted State

The following state is stored in the durable UI config:

- audio configuration
- scene configuration
- view range
- time space
- first and last key range
- active profile
- render/export defaults
- modify defaults
- merge defaults
- window position, size, maximized state, and fullscreen state
- last selected palette PNG path
- last selected background PNG path
- last selected aura PNG path

## Non-Persisted State

The following state is intentionally reset when the UI restarts or loads a new
MIDI:

- selected MIDI file
- current transport time
- playing state
- render, modify, and merge output paths
- merge source list
- job progress and terminal status
- runtime caches and snapshots

## MIDI Load Behavior

When a new MIDI file is loaded, the UI resets the render range defaults so the
export path starts cleanly from the current file:

- `start_time_text` is reset to `-1`
- `end_time_text` is reset to the loaded MIDI length

This keeps preview preroll and full-song export aligned with the loaded file
instead of preserving stale custom overrides from the previous MIDI.

## Window Restore

Window geometry is restored if the saved config contains it:

- size
- position
- maximized
- fullscreen

If the window data is missing, the app falls back to the current launch
defaults.

## Backup Recovery

The config writer uses atomic files and keeps a backup copy. If the primary
config is invalid, the UI can fall back to the backup instead of starting from
scratch.

## Practical Rule Of Thumb

If a setting changes how the app looks or behaves next time you open it, it is
probably persisted.

If a setting describes the current file, the current job, or the current play
session, it is probably not persisted.
