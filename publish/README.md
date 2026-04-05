# Linux Publish Notes

This folder holds the Linux packaging and smoke-test scripts used both locally and in GitHub Actions.

Current layout:
- `publish/linux/archive`
- `publish/linux/deb`
- `publish/linux/rpm`
- `publish/linux/appimage`
- `publish/linux/docker`
- `publish/linux/common`

Current assumptions:
- The Linux package builders run inside Docker and are intended to behave the same on local machines and GitHub Actions.
- The scripts expect the workspace dependencies to be available to Cargo in the layout used by this repo.
- Build artifacts and cargo/rustup caches live under `publish/linux/artifacts/`.

What has been revalidated:
- `tar.gz` build and smoke test
- `.deb` build and smoke test
- `.rpm` build and smoke test
- `AppImage` build and smoke test
- Shared-build Linux workflow layout for Debian-derived artifacts, so `tar.gz`, `.deb`, and `AppImage` do not each pay for a separate cold Rust build in CI

Current known issues to address:
- Pin the `AppImage` tool download more strictly, ideally with a fixed version and checksum instead of a moving URL.
- The generated Debian dependency list needed a `libasound2 | libasound2t64` style compatibility fix for Ubuntu 24.04; the broader distro dependency metadata should still be reviewed carefully.
- The package metadata is still thin:
  proper license/docs packaging should be added once the project license situation is finalized.
- `meridian-ui` is expensive to build because of the Slint/Skia stack, so CI caching and job layout will matter.
- Container-created files under `publish/linux/artifacts/` can end up root-owned; if that becomes annoying in local dev, the scripts should be adjusted to map the host UID/GID.

Current CI status:
- Linux packaging is wired into `.github/workflows/linux-publish-prototype.yml` for branch/manual validation.
- Tag-driven releases are wired into `.github/workflows/draft-release.yml`.
- The package build and smoke jobs have passed in GitHub Actions.
- The final draft-release creation step was refactored into its own job after the original inline-artifact-download approach proved awkward in the Debian job.
- A fully successful end-to-end tag push that both builds everything and creates the draft release still needs to be re-run once enough GitHub-hosted runner capacity is available.
