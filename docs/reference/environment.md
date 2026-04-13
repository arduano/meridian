# Environment Notes

This page is the low-level companion to [Troubleshooting](../troubleshooting.md).
It collects host requirements and environment variables that affect Meridian
startup, rendering, and recovery behavior.

## Host Requirements

- `ffmpeg` is required for video rendering and video-focused smoke tests.
- A usable soundfont is required for audio rendering.
- A graphics adapter/session is required for WGPU-backed frame capture and
  other headless render paths.

If any of those are missing, start with the troubleshooting page instead of
trying to debug higher-level UI or SDK code first.

## Environment Variables

| Variable | Purpose | Notes |
| --- | --- | --- |
| `MERIDIAN_SOUNDFONT` | Override the embedded default soundfont path | Used by core audio config resolution and by SDK examples/tests that want a deterministic asset |
| `MERIDIAN_TEST_SOUNDFONT` | Override the soundfont used by SDK tests/examples | Test-local convenience before falling back to the vendored asset |
| `MERIDIAN_UI_CONFIG_DIR` | Override the UI durable-config directory | Useful for isolated repros and temporary test configs |
| `MERIDIAN_FORCE_WGPU_TESTS` | Force WGPU smoke paths to run in SDK tests | Test-only override; it does not create a graphics adapter |

## Storage And Recovery

The durable UI config lives under the OS-native application config directory
unless `MERIDIAN_UI_CONFIG_DIR` is set.

Key behaviors:

- `config.json` is the primary file
- `config.json.bak` is the backup
- `config.json.tmp` is the temporary write file
- if the primary is invalid, Meridian tries the backup
- config version mismatches are treated as a signal to delete the saved UI
  config and regenerate defaults

See [UI Guide](../ui/README.md) for the user-facing behavior around remembered
settings and per-panel workflows.

## Tooling Expectations

The repo docs and smoke tests assume the current shell can run:

- `ffmpeg`
- `cargo`
- `deno`

If a test or example is failing only because one of those tools is absent, fix
the host environment first and rerun the same command.

## Related Docs

- [Testing Guide](../contributor/testing.md)
- [Video Rendering](../typescript-sdk/video-rendering.md)
- [Audio Rendering](../typescript-sdk/audio-rendering.md)
- [TypeScript SDK README](../typescript-sdk/README.md)
