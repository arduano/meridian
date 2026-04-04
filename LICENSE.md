# Meridian Licensing

This repository uses a mixed licensing model.

## Rust crates

- `crates/meridian-core`: `LGPL-3.0-or-later`
- `crates/meridian-cli`: `GPL-3.0-only`
- `crates/meridian-ui`: `GPL-3.0-only`

These license identifiers are declared directly in each crate's `Cargo.toml`.

## Repository contents outside `crates/meridian-core`

Unless a file or subdirectory states otherwise, repository contents outside `crates/meridian-core` are made available under `GPL-3.0-only`.

That includes the CLI/UI code and the rest of the repository materials that do not carry a more specific notice.

## Included license texts

Canonical license texts are bundled in this repository:

- `LICENSES/GPL-3.0.txt`
- `LICENSES/LGPL-3.0.txt`

## Third-party dependencies

Meridian also depends on third-party crates under additional licenses, including permissive licenses, `MPL-2.0`, `LGPL-3.0`, and the Slint licensing terms that make `GPL-3.0` the practical open-source distribution path for the UI.

This file summarizes Meridian's own licensing only. Third-party license obligations still apply when distributing binaries or source.
