# Windows GHA Prototype

This folder contains a conservative Windows packaging prototype intended for GitHub Actions Windows runners.

Current scope:
- build `meridian.exe` on a GitHub Actions Windows runner
- build `meridian-ui.exe` natively on Windows
- stage a portable bundle
- build a zip artifact
- build an NSIS installer
- smoke-test the staged binaries and the installed binaries on the runner

Current expectations:
- both the CLI and UI should build directly from this repo without sibling dependency checkouts
- the generated artifacts are suitable for prototype validation, not final release signing

Known limitations:
- the UI smoke test is still shallow; it checks `--help` and a short launch rather than interactive GUI behavior
