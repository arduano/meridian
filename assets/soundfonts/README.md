## Soundfonts

This directory holds redistributable soundfonts that Meridian can ship as
build inputs for examples, smoke tests, and packaged app assets.

Current default:
- `freepats-upright-kw-small/UprightPianoKW-small-20190703.sf2`

Why this one:
- piano-only
- SF2 format compatible with Meridian's xsynth backend
- explicitly CC0
- small enough to vendor in-repo and ship by default

Meridian's runtime default soundfont behavior is defined in
`crates/meridian-core/src/audio/config.rs`:
1. `MERIDIAN_SOUNDFONT`
2. embedded default `.sf2` bytes materialized to a managed temp file for xsynth

The vendored `.sf2` file in this directory is the source asset used at build
time for that embedded default.
