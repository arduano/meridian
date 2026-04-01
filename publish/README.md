# Linux Publish Notes

This folder holds the prototype Linux packaging and smoke-test scripts.

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

Known issues to address:
- Finish revalidating the full matrix after the Docker-native rewrite:
  `tar.gz`, `.deb`, `.rpm`, and `AppImage`.
- Pin the `AppImage` tool download more strictly, ideally with a fixed version and checksum instead of a moving URL.
- The generated Debian dependency list needed a `libasound2 | libasound2t64` style compatibility fix for Ubuntu 24.04; the broader distro dependency metadata should still be reviewed carefully.
- RPM and AppImage flows need another clean smoke pass after the latest builder changes.
- The package metadata is still thin:
  proper license/docs packaging should be added once the project license situation is finalized.
- `meridian-ui` is expensive to build because of the Slint/Skia stack, so CI caching and job layout will matter.
- Container-created files under `publish/linux/artifacts/` can end up root-owned; if that becomes annoying in local dev, the scripts should be adjusted to map the host UID/GID.
- The scripts currently use direct Docker CLI orchestration.
  If we want nicer CI logs or matrix control, that can be wrapped in a GitHub Actions workflow later without changing the package scripts themselves.
