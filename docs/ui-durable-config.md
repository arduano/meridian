# Durable Config For `meridian-ui`

## Current state

`meridian-ui` does not currently have an app-level durable config system.

- Startup state is rebuilt from `UiOptions` defaults and CLI flags.
- `UiCoreBridge::initialize` then pushes defaults into the core with `SetAudioConfig`, `SetSceneConfig`, `SetViewRange`, `SetKeyRange`, `SetViewport`, and `SetTime`.
- Most user actions mutate either core state or Slint UI properties only.
- I did not find any config/data directory lookup, config file load, config file save, or migration layer.
- The only "remembered" asset paths are process-local statics (`LAST_PALETTE_PNG`, `LAST_AURA_PNG`, `LAST_BACKGROUND_PNG`), so they disappear on restart.

## What is already serializable

The good news is that the backend-facing state is already close to persistable:

- `meridian_core::protocol::StateSnapshot` is `Serialize`/`Deserialize`.
- `AudioConfig` is `Serialize`/`Deserialize`.
- `SceneConfig` is `Serialize`/`Deserialize`.
- Merge and modify configs are already structured Rust data with serde support.

That means Meridian does not need a bespoke stringly-typed settings layer. It should persist a curated subset of the existing typed config.

## What should persist

### Global durable preferences

These should persist automatically across restarts:

- Audio engine preferences:
  sample rate, channel count, render window, soundfont path, interpolation, effects, layer limit, threading, ignore range, envelope curves, limiter.
- Display / renderer preferences:
  renderer choice, scene config, palette/background/aura asset selections, view range, time-space mode, key range.
- Export defaults:
  mode, resolution, fps, codec, CRF, preset, pixel format, RGB mode, alpha export toggle, audio bitrate/format/sample rate/channel count, ffmpeg arg text, open-after-export.
- Panel defaults:
  merge layout / metadata / PPQ override, last modify pass, last valid modify JSON.
- Desktop window state:
  size, position, maximized/fullscreen state, and possibly sidebar/panel layout once that exists.
- Convenience history:
  recent MIDI files, recent output directories, recent asset directories.

### Session restore state

This is different from preferences and should be treated as restorable session data:

- Last opened MIDI path.
- Last transport time.
- Modify output path.
- Merge source list and merge output path.
- Render output path.

Session restore should be resilient:

- Missing files should be ignored, not fatal.
- In-progress jobs, progress meters, live analysis results, and active IDs should never be restored.

## What should not persist directly

Do not write raw `StateSnapshot` straight to disk and reload it blindly.

`StateSnapshot` contains ephemeral runtime state that is invalid after restart:

- active resource IDs
- active job IDs
- `audio_status`
- live viewport dimensions derived from current session
- `playing`
- analysis/job outputs

Instead, define a persistence schema that copies only the restart-safe fields.

## Recommended design

## 1. Add a typed app persistence schema

Create a new module in `meridian-ui`, for example:

- `crates/meridian-ui/src/config/mod.rs`
- `crates/meridian-ui/src/config/schema.rs`
- `crates/meridian-ui/src/config/store.rs`

Recommended top-level shape:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct UiConfigFile {
    pub version: u32,
    pub preferences: UiPreferences,
    pub session: UiSessionRestore,
}
```

Suggested split:

- `UiPreferences`
  durable cross-project user preferences
- `UiSessionRestore`
  last session / working context

This separation matters. Users expect audio/render preferences to stick, but may not want every project/session reopened forever.

## 2. Use JSON, not TOML

Use pretty JSON for the persisted config file.

Why:

- `serde_json` is already in the workspace.
- Existing backend config types already round-trip cleanly with serde.
- Meridian already has JSON-oriented config/editor surfaces in the modify panel.
- JSON handles tagged enums like `SceneConfig` more cleanly than TOML.

Suggested files:

- config file: `config.json`
- backup file: `config.json.bak`

## 3. Use OS-native app directories

Add a small directory helper using a crate like `directories` or `directories-next`.

Suggested base identity:

- qualifier: `io`
- organization: `github.arduano`
- application: `Meridian`

Expected directories:

- Linux: `${XDG_CONFIG_HOME:-~/.config}/Meridian`
- macOS: `~/Library/Application Support/Meridian`
- Windows: `%APPDATA%\\Meridian`

Suggested layout:

- config dir:
  `config.json`
  `config.json.bak`
- data dir:
  future caches/session artifacts if needed

## 4. Load config before `initialize_core`

The current startup flow creates the app and immediately calls `initialize_core`.

Instead:

1. Create the app.
2. Load `UiConfigFile`.
3. Merge persisted defaults with CLI overrides.
4. Apply persisted Slint-only UI state.
5. Call `initialize_core` using the merged state.
6. If session restore has a MIDI path and it still exists, load it.
7. Seek to the restored time after MIDI load succeeds.

CLI should remain highest priority for that launch.

## 5. Persist from typed state, not from scattered callbacks

Avoid saving directly inside every callback.

Use a small in-memory `ConfigController` that owns:

- latest `UiConfigFile`
- dirty flag
- debounce timer

Update the controller from:

- core snapshot changes
- Slint-only export/modify/merge changes
- window move/resize/maximize events

Then save with debounce, for example 300-1000 ms after the last mutation.

This keeps writes small and avoids dozens of disk writes while sliders are moving.

## 6. Save atomically

For desktop-grade durability, writes should be crash-safe:

1. Serialize to `config.json.tmp`.
2. Flush file contents.
3. Rename current `config.json` to `config.json.bak` if present.
4. Rename temp file to `config.json`.

On startup:

1. Try `config.json`.
2. If invalid, try `config.json.bak`.
3. If both fail, log and fall back to defaults.

This is much closer to what users expect from a native desktop app.

## 7. Version and migrate the file

Include `version` from day one.

Even if v1 only has one schema, this avoids painting the app into a corner once scene/render options evolve.

Migration rule:

- load older versions into compatibility structs
- convert forward into current `UiConfigFile`
- rewrite current format on next save

## Suggested v1 field set

This is the subset I would ship first:

```rust
pub struct UiPreferences {
    pub disable_wgpu: bool,
    pub audio: AudioConfig,
    pub scene: SceneConfig,
    pub view_range: f64,
    pub time_space: DisplayTimeSpace,
    pub first_key: u8,
    pub last_key: u8,
    pub export: ExportPreferences,
    pub modify: ModifyPreferences,
    pub merge: MergePreferences,
    pub recent_midis: Vec<PathBuf>,
    pub recent_dirs: RecentDirectories,
    pub window: WindowPreferences,
}

pub struct UiSessionRestore {
    pub reopen_last_project: bool,
    pub midi_path: Option<PathBuf>,
    pub current_time: f64,
    pub render_output_path: Option<PathBuf>,
    pub modify_output_path: Option<PathBuf>,
    pub modify_config_text: Option<String>,
    pub merge_sources: Vec<PathBuf>,
    pub merge_output_path: Option<PathBuf>,
}
```

Notes:

- `AudioConfig` and `SceneConfig` should be stored directly.
- `playing` should not persist.
- `current_time` should be clamped to the restored MIDI length on load.
- If a stored path no longer exists, drop it and continue.

## Mapping to current code

The lowest-risk implementation path is:

- Load/save config in `run_ui` startup/shutdown flow.
- Derive persisted core preferences from `shared_state.snapshot`.
- Derive Slint-only preferences from app properties.
- Replace the in-process `LAST_*` statics with values sourced from the persisted config/session state.

Concrete current boundaries:

- startup defaults come from `UiOptions` and `UiCoreBridge::initialize`
- durable audio settings already funnel through `AudioConfig`
- durable scene settings already funnel through `SceneConfig`
- export preferences currently live only in Slint properties
- merge and modify settings currently live only in Slint properties / UI model state

## Rollout plan

### Phase 1

Implement durable preferences only:

- audio config
- scene config
- view range / key range / time-space
- export defaults
- window geometry
- last asset paths

### Phase 2

Add session restore:

- last MIDI
- last time position
- modify/merge/export output paths
- merge sources
- modify JSON

### Phase 3

Add UX polish:

- recent files menu
- "Restore previous session" toggle
- "Reset preferences" action
- config import/export for sharing render setups

## Short recommendation

The repo is already set up for typed persistence. The right move is not a generic settings map and not raw `StateSnapshot` persistence. The right move is:

- typed `UiConfigFile`
- JSON storage
- OS-native config dir
- atomic save with backup
- explicit split between durable preferences and restorable session state
- CLI overrides applied on top at startup

That will get Meridian to a desktop-grade persistence model without fighting the current architecture.
