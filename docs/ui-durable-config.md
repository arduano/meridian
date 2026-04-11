# Durable Config For `meridian-ui`

`meridian-ui` now uses a typed, preferences-only durable config.

## Current shape

The config file is:

```json
{
  "version": 1,
  "preferences": { ... }
}
```

There is no `session` key.

Persisted preferences currently include:

- audio config
- scene config
- view range / time-space / key range
- active profile
- export panel defaults
- modify panel defaults
- merge panel defaults
- window geometry/state
- last selected palette/background/aura asset paths

Not persisted:

- selected MIDI file
- transport time
- render / modify / merge output paths
- merge source list
- playing state
- job status / progress
- runtime ids / caches / snapshots

## Storage

The store lives in the OS-native app config directory using `directories`:

- Linux: `${XDG_CONFIG_HOME:-~/.config}/meridian/config.json`
- macOS: `~/Library/Application Support/Meridian/config.json`
- Windows: `%APPDATA%\\Meridian\\config.json`

Override:

- `MERIDIAN_UI_CONFIG_DIR=/some/path`

Writes are atomic:

- `config.json.tmp`
- `config.json.bak`
- `config.json`

Startup loads `config.json` first, then falls back to `config.json.bak` if the primary file is invalid.

## Runtime behavior

Startup flow:

1. Load config.
2. Merge persisted preferences with CLI overrides.
3. Apply window preferences.
4. Initialize core from the merged startup options.
5. Restore Slint-only panel preferences.
6. Install debounced autosave.

CLI flags still win for the current launch.

## Missing asset paths

Missing persisted PNG paths are not all handled the same way today:

- Aura PNG:
  invalid or missing path falls back to the builtin ring aura.
- Background PNG:
  invalid or missing path logs an error and renders without the background texture.
- Palette PNG:
  invalid or missing path propagates as a palette build error from the core.

That means aura and background fail soft, while palette is still the sharp edge if the persisted scene references a bad PNG.

## Code layout

The persistence implementation is split under:

- `crates/meridian-ui/src/ui/runtime/persistence/schema.rs`
- `crates/meridian-ui/src/ui/runtime/persistence/store.rs`
- `crates/meridian-ui/src/ui/runtime/persistence/startup.rs`
- `crates/meridian-ui/src/ui/runtime/persistence/sync.rs`
- `crates/meridian-ui/src/ui/runtime/persistence/assets.rs`

`sync.rs` stays under the 500-line business-logic limit and uses small binding macros so export/merge field mappings only live once for capture + restore.
