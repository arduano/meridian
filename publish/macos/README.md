# macOS GHA Prototype

This folder contains a conservative macOS packaging prototype intended for GitHub Actions macOS runners.

Current scope:
- build `meridian` and `meridian-ui` natively on GitHub Actions
- stage a portable directory with the built binaries
- build a zip artifact for each runner architecture
- smoke-test the CLI and UI binaries on the runner

Current expectations:
- the workflow should build on both Intel and Apple Silicon GitHub runners
- the artifacts are suitable for prototype validation, not signing or notarization

Known limitations:
- the UI smoke test is still shallow; it checks `--help` and a short launch rather than interactive GUI behavior
- this does not produce a signed `.app` bundle yet
