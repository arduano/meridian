# Workflow Notes

This directory currently has two kinds of workflows:

- `draft-release.yml`
  Builds the release artifacts for Linux, Windows, and macOS on tag push and creates or updates a GitHub draft release.
- `*-prototype.yml`
  Manual or branch-scoped validation workflows that are useful when iterating on packaging logic without touching the tag-driven release flow.

## Current release shape

`draft-release.yml` currently does this:

- Linux Debian job builds and smoke-tests `tar.gz`, `.deb`, and `AppImage`
- Linux RPM job builds and smoke-tests `.rpm`
- Windows job builds, smoke-tests staged binaries, builds the installer, then smoke-tests the installed binaries
- macOS jobs build `x86_64` and `arm64` portable zips and smoke-test both CLI and UI entrypoints
- A final `draft-release` job waits for all artifact-producing jobs and then uses `actions/download-artifact` to assemble the GitHub draft release

The release creation used to live inside the Debian Linux job. That shape made the Debian job look "stuck" while it was really waiting on Windows/macOS artifacts. The current dedicated final job is the intended layout going forward.

## What has been verified

- Linux package build + Docker smoke matrix has passed for `tar.gz`, `.deb`, `.rpm`, and `AppImage`
- Windows build + staged smoke + installer smoke has passed
- macOS Intel and Apple Silicon build + smoke has passed

Those validations were done in GitHub Actions runs before the final release job was split out. The split-out `draft-release` job itself still needs one fully funded end-to-end tag run to confirm draft release creation and asset attachment.

## Current blocker

For a private repository, these workflows can exhaust GitHub-hosted Actions minutes quickly, especially because Windows and macOS UI builds are still relatively expensive. Once the repository is public, standard GitHub-hosted runners should no longer consume billed Actions minutes, which is the expected long-term setup for this project.

## Operational notes

- Artifact retention is intentionally short for these workflows to avoid unnecessary storage usage during repeated dry runs.
- Prototype workflows are still useful for packaging iteration, but the tag-driven release path should be considered the source of truth.
- The Node 20 deprecation warnings from `actions/checkout@v4`, `actions/cache@v4`, and `actions/upload-artifact@v4` should be cleaned up later.
