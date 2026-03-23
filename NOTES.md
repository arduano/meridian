# NOTES

## Expected blockers on a headless NAS

- A desktop Slint app still needs a working windowing/display stack. On this NAS/SSH session, the verified current runtime failure is exactly that: `neither WAYLAND_DISPLAY nor WAYLAND_SOCKET nor DISPLAY is set`.
- If no GPU device nodes are exposed into the runtime environment, `wgpu` may fail adapter selection entirely after the display issue is solved.
- Even with a GPU present, remote SSH sessions often lack `DISPLAY`, `WAYLAND_DISPLAY`, `XDG_RUNTIME_DIR`, or permissions for DRM/render nodes.

## Intel Arc specific expectations

- Intel Arc should work in principle through Vulkan or other native backends that `wgpu` can select, but driver maturity and permissions matter.
- On Linux, confirm access to `/dev/dri/renderD*` and the presence of working Mesa/Intel userspace.
- If the system falls back to software rendering, the proof of concept may still open, but it would not represent the intended accelerated path.

## Remote SSH / container / NAS caveats

- `cargo run` over plain SSH without X forwarding or a compositor usually cannot open the Slint window.
- X11 forwarding may allow launch but can still expose GPU acceleration or presentation issues.
- Wayland forwarding or compositor remoting can change frame pacing and introduce presentation problems that do not appear on a local desktop session.

## What to try later on the NAS

1. Enter the provided `nix-shell` first so `fontconfig`, `pkg-config`, and the common Linux graphics libs are present, but prefer the host Rust toolchain on this machine (`PATH=/run/current-system/sw/bin:$PATH`) because `wgpu` 28 wants newer Rust than the Nix-shell default here.
2. Verify that a graphical session or virtual display exists.
3. Verify that the user can access GPU render nodes.
4. Try `cargo check` first, then `cargo run`.
5. If window creation fails, try again under a local desktop login or a controlled Xvfb/Wayland test setup.
6. If adapter creation fails, inspect driver/runtime access before changing application code.
