## Soundfonts

This directory holds redistributable SFZ soundfonts that Meridian can ship as
bundled defaults for examples, smoke tests, and packaged app assets.

Current default:
- `freepats-upright-kw-small/UprightPianoKW-small-20190703.sfz`

Why this one:
- piano-only
- SFZ format compatible with Meridian's xsynth backend
- explicitly CC0
- small enough to vendor in-repo and ship by default

Meridian's runtime default soundfont path is resolved in
`crates/meridian-core/src/audio/config.rs` using this order:
1. `MERIDIAN_SOUNDFONT`
2. cwd-relative bundled asset path
3. executable-adjacent bundled asset path
4. final fallback to the relative asset path string
