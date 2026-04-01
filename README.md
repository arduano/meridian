# meridian

Seed repository for the new Rust rewrite direction: starting from the Slint + embedded `wgpu` proof of concept that will become a new Zenith-inspired application.

## Architecture

The app is designed around Slint's documented texture-import path:

- Slint owns the desktop window and normal surrounding UI layout.
- Rust code hooks into `Window::set_rendering_notifier()`.
- The notifier receives Slint's active shared `wgpu::Device` and `wgpu::Queue`.
- Native Rust code allocates an off-screen `wgpu::Texture`, renders a procedural animated shader into it, and imports that texture into Slint with `slint::Image::try_from(texture)`.
- The imported image is displayed in a clearly bounded viewport rectangle inside the Slint scene.

This is accelerated texture compositing inside the Slint window, not an HTML/WebGPU view and not a separate native child swapchain surface.

## Current status

The source code in [src/main.rs](/home/arduano/programming/meridian/src/main.rs) is written against Slint's public `unstable-wgpu-28` integration API as documented by Slint. It demonstrates:

- surrounding Slint UI
- a bordered embedded viewport rectangle
- shared-device native `wgpu` rendering
- texture import back into Slint layout
- resize-aware viewport texture recreation
- continuous redraw requests for animation

The Rust source now verifies successfully with:

```bash
cargo fmt
nix-shell --run 'PATH=/run/current-system/sw/bin:$PATH cargo check'
```

A `shell.nix` is included to supply the expected native Linux development libraries on NixOS-friendly machines while still allowing the host Rust toolchain to drive the build.

## Commands

If you use `direnv`, run `direnv allow` once in this repo. The included [`.envrc`](/home/arduano/programming/meridian/.envrc) loads the existing [`shell.nix`](/home/arduano/programming/meridian/shell.nix) environment and prepends `/run/current-system/sw/bin` so the host Rust toolchain remains preferred.

Build/check (recommended on this host):

```bash
nix-shell --run 'PATH=/run/current-system/sw/bin:$PATH cargo check'
```

Run the CLI:

```bash
nix-shell --run 'PATH=/run/current-system/sw/bin:$PATH cargo run -p meridian-cli -- --help'
```

Run stdio mode:

```bash
nix-shell --run 'PATH=/run/current-system/sw/bin:$PATH cargo run -p meridian-cli -- stdio'
```

Run without the accelerated viewport (A/B resize test):

```bash
MERIDIAN_DISABLE_WGPU=1 nix-shell --run 'PATH=/run/current-system/sw/bin:$PATH cargo run'
```

Backend comparison commands (still with `MERIDIAN_DISABLE_WGPU=1`):

```bash
# Default backend/renderer choice
MERIDIAN_DISABLE_WGPU=1 nix-shell --run 'PATH=/run/current-system/sw/bin:$PATH cargo run'

# Winit + FemtoVG
MERIDIAN_DISABLE_WGPU=1 SLINT_BACKEND=winit-femtovg nix-shell --run 'PATH=/run/current-system/sw/bin:$PATH cargo run'

# Winit + software renderer
MERIDIAN_DISABLE_WGPU=1 SLINT_BACKEND=winit-software nix-shell --run 'PATH=/run/current-system/sw/bin:$PATH cargo run'

# Winit + Skia renderer
MERIDIAN_DISABLE_WGPU=1 SLINT_BACKEND=winit-skia nix-shell --run 'PATH=/run/current-system/sw/bin:$PATH cargo run'

# If Qt is installed and available
MERIDIAN_DISABLE_WGPU=1 SLINT_BACKEND=Qt nix-shell --run 'PATH=/run/current-system/sw/bin:$PATH cargo run'
```

## What was attempted on this host

Attempted directly on this host:

```bash
cargo check
cargo run
nix-shell --run 'PATH=/run/current-system/sw/bin:$PATH cargo check'
nix-shell --run 'PATH=/run/current-system/sw/bin:$PATH cargo run'
```

Observed results:

- Plain host `cargo check` reached `crates.io`, but failed on missing native Linux development dependencies (`fontconfig` via `pkg-config`).
- A first `nix-shell` retry fixed those native libraries, but the Nix-shell-provided Rust toolchain was too old for `wgpu` 28.
- Using the included `shell.nix` for native libraries **plus** the host Rust toolchain (`cargo 1.94` / `rustc 1.94`) succeeded for `cargo check`.
- `cargo run` then reached actual runtime initialization and failed at the expected headless boundary: there is no active X11 or Wayland display in this NAS/SSH session.

Verified successful build command:

```bash
nix-shell --run 'PATH=/run/current-system/sw/bin:$PATH cargo check'
```

Verified runtime failure on this host:

```text
Error: Error initializing winit event loop: ... neither WAYLAND_DISPLAY nor WAYLAND_SOCKET nor DISPLAY is set.
```

So the current blocker is no longer compilation — it is the lack of a real desktop/display session for actually opening the Slint window.

## Headless and NAS notes

See [NOTES.md](/home/arduano/programming/meridian/NOTES.md) for expected runtime blockers on headless NAS, Intel Arc, remote SSH, Wayland/X11, and display-server availability.
