## MIDI Fixtures

This directory holds small, redistributable MIDI fixtures for non-UI examples,
CLI smoke flows, and integration tests.

Layout:
- `piano/`: curated solo-piano pieces sourced from Mutopia
- `test/`: reserved for tiny deterministic fixtures if the test suite needs
  stricter edge-case coverage than the public pieces provide

Current curated piano set:
- `piano/burgmuller-op100-no4-the-little-party.mid`
- `piano/burgmuller-op100-no13-consolation.mid`
- `piano/mozart-kv457-sonata-no14-fragment.mid`
- `smoke-two-notes.mid`

Selection criteria:
- royalty-free redistribution surface
- small enough for repository-friendly smoke tests
- piano-only material suitable for CLI and SDK examples
- musically varied enough to exercise analysis, processing, and rendering paths

Guidelines:
- prefer reusing these files across Rust, CLI, and TypeScript tests
- write generated outputs to temp directories, not back into this tree
- do not add large rendered audio/video artifacts to git

See `PROVENANCE.md` for original source URLs and licensing notes.
