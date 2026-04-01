# Windows GHA Prototype

This folder contains a conservative Windows packaging prototype intended for GitHub Actions Windows runners.

Current scope:
- build `meridian.exe` on `windows-latest`
- attempt to build `meridian-ui.exe` natively on Windows
- stage a portable bundle
- build a zip artifact
- build an NSIS installer
- smoke-test the built CLI and the installed CLI on the runner

Current expectations:
- `meridian.exe` should be the primary success path
- `meridian-ui.exe` is best-effort until the Windows GUI toolchain story is confirmed end to end
- the workflow checks out `midi-toolkit-rs` and `xsynth` as sibling directories because the workspace currently uses sibling path dependencies

Known limitations:
- the UI smoke test is intentionally shallow; it only verifies that the binary can be launched and kept alive briefly
- the generated artifacts are suitable for prototype validation, not final release signing or notarization

Progress log:
- A throwaway branch was created and pushed for GitHub Actions validation:
  - branch: `codex/windows-gha-prototype-20260401-121216`
  - commit: `116a11a284cc615659b4a926b41a55f4458db496`
- The workflow file for that branch is:
  - `.github/workflows/windows-prototype.yml`
- The first Windows runner execution was:
  - run id: `23827124509`
  - job id: `69452540531`
- The run reached the native Windows build phase successfully and then failed in `Build CLI`.
- No runner artifacts were uploaded because the failure happened before packaging.

Observed GitHub Actions failure:
- The failure was not caused by the Windows packaging scripts.
- It failed while compiling `meridian-core` on the runner because the checked out sibling dependency repos did not match the local dependency state this Meridian checkout currently expects.
- Concrete missing APIs on the runner:
  - `xsynth_realtime::DefaultOutputSupport`
  - `xsynth_realtime::RealtimeSynth::default_output_support()`
  - `xsynth_realtime::RealtimeSynth::open_with_default_output_and_params()`
- The runner checked out:
  - `midi-toolkit-rs` from remote `master` at `d77c82d61e3e58fe51da9bd55099fecad41b2e8c`
  - `xsynth` from remote `master` at `bc96ebba070cf8bcfd29a55e37b59ea97f0a6924`

Local dependency state at time of investigation:
- Local `xsynth` checkout:
  - path: `/home/arduano/programming/xsynth`
  - branch: `meridian-features`
  - HEAD: `96f613987de019494afebb484be1985f20961775`
  - local modifications were present
- Local `midi-toolkit-rs` checkout:
  - path: `/home/arduano/programming/midi-toolkit-rs`
  - branch: `master`
  - HEAD: `d77c82d61e3e58fe51da9bd55099fecad41b2e8c`
  - local modifications were present
- The local `xsynth` checkout contains the APIs Meridian needs, but those changes are not available from the public remote refs currently used by the workflow.
- The remote `xsynth` repo did not have a `meridian-features` branch during this attempt.

Implication:
- A fresh GitHub Actions Windows runner cannot currently reproduce the same dependency state as the local machine.
- Until the required sibling dependency state is pushed to reachable git refs, the workflow should be expected to fail before artifact creation.

Next steps for the next agent:
- Do not spend time debugging Windows packaging first; the current blocker is dependency reproducibility.
- Once usable refs exist for `xsynth` and, if needed, `midi-toolkit-rs`, update `.github/workflows/windows-prototype.yml` to pin those exact refs in the checkout steps.
- Rerun the same workflow on the throwaway branch or a new temporary branch.
- After the build succeeds, download the uploaded artifacts and validate:
  - portable zip contents
  - NSIS installer generation
  - runner-side CLI smoke output
  - whether `meridian-ui.exe` built at all on native Windows
