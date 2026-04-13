# Docs

This directory is the main documentation index for Meridian. The structure is
progressive on purpose:

1. get one working path running
2. learn the main user-facing workflows
3. move into advanced knobs and troubleshooting
4. use contributor docs only when you need codebase orientation

## Start Here

- [Getting Started](./getting-started.md): first successful CLI, UI, and SDK
  paths
- [Repository README](../README.md): workspace overview, requirements, and
  validation matrix

## Workflow Guides

- [CLI Guide](./cli/README.md): command-line workflows from quickstart to
  advanced flags
- [UI Guide](./ui/README.md): loading MIDI, playback, modify, merge, export,
  and persistence behavior
- [TypeScript SDK](./typescript-sdk/README.md): programmatic control through the
  checked Deno-first SDK path

## Reference And Troubleshooting

- [Troubleshooting](./troubleshooting.md): common setup, rendering, and config
  failure modes
- [Environment Reference](./reference/environment.md): environment variables
  and host requirements that affect rendering, tests, and persistence

## Contributor Docs

- [Contributor Docs](./contributor/README.md): contributor-oriented map and
  validation entrypoints
- [Starting Points](./contributor/starting-points.md): contributor map for
  core, UI, CLI, SDK, rendering, and tests
- [Testing Guide](./contributor/testing.md): focused validation commands by
  subsystem
