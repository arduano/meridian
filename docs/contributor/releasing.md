# Releasing

Meridian ships with one version across all public surfaces:

- `meridian-core`
- `meridian-cli`
- `meridian-ui`
- the TypeScript/Deno SDK

The release truth lives in the root [`VERSION`](../../VERSION) file. Rust crate
versions, the private SDK package metadata, Git tags, and GitHub Releases all
flow from that value.

## Version Model

- source of truth: [`VERSION`](../../VERSION)
- release tag: `vX.Y.Z`
- Rust workspace version: [`Cargo.toml`](../../Cargo.toml)
- SDK metadata version: [`sdk/typescript/package.json`](../../sdk/typescript/package.json)
- Nix package version: derived from `VERSION` in [`flake.nix`](../../flake.nix)

For shipped SDK consumers, the meaningful version is the Git tag they import
from, not a registry package version.

## Version Commands

Set the release version everywhere:

```bash
scripts/set-version 0.2.0
```

Check that all release-facing version fields agree:

```bash
scripts/check-version
```

You can also validate an expected release tag value directly:

```bash
scripts/check-version v0.2.0
```

## Release Prep

Generate a release draft from the current branch:

```bash
scripts/prepare-release
```

Or set the version and prepare in one step:

```bash
scripts/prepare-release 0.2.0
```

That script:

- verifies the worktree is clean
- checks version consistency
- drafts release notes at `target/release/release-notes-vX.Y.Z.md`
- includes the commit range since the previous `v*` tag

The draft notes are intentionally editable. The intended flow is:

1. generate the draft
2. trim or rewrite the top summary
3. optionally use an LLM to polish the summary and upgrade notes
4. keep the underlying commit list and release surface accurate

## Tag And Publish

Once the version bump and release notes are ready:

```bash
git tag v0.2.0
git push origin main
git push origin v0.2.0
```

Pushing the `v*` tag triggers
[`draft-release.yml`](../../.github/workflows/draft-release.yml), which:

- verifies the tag matches `VERSION`
- builds the release artifacts
- creates or updates a draft GitHub Release
- lets GitHub generate categorized release notes using
  [`.github/release.yml`](../../.github/release.yml)

## SDK Publishing Model

The SDK is not published to JSR or npm.

The public distribution model is:

- source lives in GitHub
- binaries/installers live in GitHub Releases
- Deno/TypeScript users import from a Git tag

Use the stable entrypoint:

```ts
import { createDenoMeridianClient } from "https://raw.githubusercontent.com/<owner>/meridian/v0.2.0/sdk/typescript/mod.ts";
```

Do not document imports from `main`. Use release tags or pinned commits.
