# Blue Retro Slint + `wgpu` App Guide

This document is the handoff guide for turning the current proof of concept into a real application.

The intended direction is now clear:

- **native Rust desktop app**
- **Slint for the surrounding application UI**
- **embedded Rust `wgpu` viewport inside the Slint window**
- **visual direction: soft blue retro workstation / CRT-inspired UI**
- **not web UI, not Tauri, not HTML/CSS, not a separate child graphics window**

---

## 1. Product thesis

Build a desktop app where:

- Slint owns the window chrome, panels, tabs, forms, lists, status UI, and app structure.
- A bounded central region hosts a real accelerated `wgpu` view.
- The app feels like a **calm retro-blue workstation**, not a neon toy and not a default enterprise dashboard.
- The 3D/graphics view is a first-class part of the app, not a bolt-on gimmick.

The design target is roughly:

- **blue retro**
- **soft, not harsh**
- **workstation / terminal heritage**
- **clean enough for a modern tool**
- **native and authored, not webby**

---

## 2. Core architectural principle

Keep the architecture split cleanly into two layers:

## Slint layer

Responsible for:

- app window
- layout
- sidebars / inspectors / toolbars / tabs
- text and controls
- settings panels
- status overlays
- command surfaces around the viewport

## `wgpu` layer

Responsible for:

- render pipeline
- scene state / camera / meshes / shaders
- off-screen texture rendering
- GPU resource lifecycle
- redraw scheduling
- viewport-only interaction/render logic

## Bridge between them

Slint and `wgpu` should communicate through a small, explicit bridge:

- Slint sends user intent into Rust app state
- Rust updates render state
- renderer draws into a texture
- texture is imported back into Slint as an image

That split is the main thing worth preserving.

---

## 3. Keep this rendering strategy

The correct pattern, based on the PoC, is:

1. Slint selects the `wgpu` backend via `require_wgpu_28(...)`
2. Use `Window::set_rendering_notifier(...)`
3. Receive Slint's shared `wgpu::Device` and `wgpu::Queue`
4. Render your own off-screen `wgpu::Texture`
5. Import that texture into Slint with `slint::Image::try_from(texture)`
6. Show it in a bounded layout region

This is the key decision.

## Why this is the right path

- one native window
- one coherent layout system
- no HTML bridge
- no WebGPU frontend stack
- no separate child swapchain surface fighting the UI toolkit
- app chrome and viewport feel like one product

---

## 4. UI direction to preserve

The chosen direction is **Blue CRT workstation**.

That means:

### Palette

Use:

- deep navy backgrounds
- desaturated blue-gray surfaces
- soft icy-blue accents
- slightly misty text tones
- low amounts of pure white
- very limited warm accent use

Avoid:

- bright phosphor green terminal look
- overly saturated cyberpunk magenta/cyan everywhere
- generic light SaaS dashboard look
- default “dark mode app” with random blue highlight

### Tone

The UI should feel like:

- a serious tool
- a little nostalgic
- calm and legible
- technical without being aggressive

Good keywords:

- workstation
- CRT
- terminal heritage
- instrumentation
- calm ops console
- blue steel
- night desk

Bad keywords:

- gamer RGB
- hacker movie neon
- dribbble glass candy
- generic enterprise admin panel

---

## 5. Layout principles

One of the biggest lessons from this PoC is that the app should avoid “stretch everything to fill the pane.”

## Preferred layout behavior

- fixed-ish or bounded content widths for inspector-style UI
- natural content height where possible
- extra vertical room should pool **below** the content, not inflate internal controls
- top-aligned panels are better than vertically smeared panels
- the viewport itself can fill available area; control cards generally should not

## Avoid

- forcing internal cards to fill parent height
- stacking many hidden demo children in a layout
- mixing absolute positioning with layout-managed children unless very deliberate
- stretching controls just because a pane is tall

## Good pattern

For real screens, prefer:

- top toolbar
- left sidebar or tool rail
- central embedded `wgpu` viewport
- right inspector / properties panel
- bottom status strip if needed

That is a more realistic app shell than big free-floating cards.

---

## 6. Recommended app skeleton

If building from scratch, structure the app more like this:

```text
AppWindow
├─ TopBar
│  ├─ app title / project name
│  ├─ workspace tabs
│  ├─ quick actions
│  └─ status indicators
├─ Body
│  ├─ LeftToolRail
│  │  ├─ mode buttons
│  │  ├─ scene/navigation tools
│  │  └─ selection / creation tools
│  ├─ MainViewportArea
│  │  ├─ viewport header / breadcrumbs
│  │  ├─ embedded wgpu viewport
│  │  └─ transient overlays / HUD
│  └─ RightInspector
│     ├─ object/scene properties
│     ├─ toggles/sliders
│     ├─ tabs for render/camera/selection
│     └─ logs or metadata
└─ BottomStatusBar
   ├─ FPS
   ├─ camera mode
   ├─ selection info
   └─ render/backend status
```

This is the form factor the blue-retro theme should ultimately serve.

---

## 7. Styling system to introduce next

Right now the PoC is still too hand-authored and ad hoc. The next real step is a small design system.

Create reusable primitives for:

- panel
- section header
- toolbar button
- segmented control
- inspector row
- labeled numeric control
- toggle chip
- badge/status pill
- viewport overlay card
- status bar item

Each primitive should accept a small set of theme tokens rather than hardcoded colors.

## Suggested theme token set

At minimum:

- `bg_app`
- `bg_panel`
- `bg_panel_alt`
- `bg_control`
- `border_soft`
- `border_strong`
- `text_main`
- `text_muted`
- `text_dim`
- `accent_primary`
- `accent_secondary`
- `accent_success`
- `shadow_tint`

If Slint theming feels awkward globally, keep the theme values in Rust and pass them down explicitly at first.

---

## 8. Typography guidance

Typography is the next missing piece.

The current PoC mostly proves layout and color. To make the app feel authored, upgrade type hierarchy.

## Desired typography character

- slightly technical
- crisp
- restrained
- more workstation than playful dashboard

Use hierarchy like:

- app title: strong and compact
- panel title: medium-bold
- labels: small and muted
- values: brighter and more precise
- status/meta text: dimmer and denser

If custom font loading is practical, explore:

- a modern monospace or quasi-monospace for status/values
- a clean sans for normal UI text

Even without custom fonts, stronger hierarchy will help a lot.

---

## 9. `wgpu` viewport guidance

The embedded viewport should become a real subsystem, not stay as a shader toy.

## Separate these concerns in Rust

Create modules like:

- `app_state.rs`
- `ui_state.rs`
- `render_state.rs`
- `viewport_renderer.rs`
- `scene.rs`
- `camera.rs`
- `gpu_resources.rs`

## The renderer should own

- pipeline setup
- uniform/state buffers
- texture lifecycle
- resize handling
- redraw behavior
- render passes

## UI should own

- selected mode
- inspector values
- toggles/sliders
- tool selection
- which overlays are visible

Do not bury app logic inside the rendering notifier closure longer-term.

---

## 10. Interaction model

A real app should define interaction boundaries early.

## Good split

### Slint handles

- tabs
n- buttons
- inspector controls
- app navigation
- status UI

### Viewport handles

- camera orbit / pan / zoom
- scene interaction
- hit testing / picking
- visual overlays inside the render area

That avoids trying to force every viewport interaction into normal UI widgets.

---

## 11. Resize / platform lessons already learned

Important lesson from this exploration:

On Leo's KDE X11 stack, Slint live resizing behaved badly across multiple tested renderers/backends, and not just when the embedded `wgpu` path was active.

So treat this as an active platform caveat.

## Practical implication

When turning this into a real app:

- prefer testing on both **Wayland and X11**
- do not assume resize performance issues are caused by the embedded viewport
- keep the app shell resilient even if live-resize polish is imperfect for now

Also:

- avoid gratuitous layout thrash during resize
- keep expensive recalculation separated from tiny UI state changes

---

## 12. Build plan from here

Recommended implementation order:

### Phase 1 — clean project extraction

Create a fresh app repo or new app target with:

- Slint app shell
- embedded `wgpu` viewport path copied from the PoC
- blue-retro theme tokens
- one realistic screen layout

### Phase 2 — replace demo gallery with real app shell

Delete the demo-style showcase framing and build:

- top toolbar
- left tool rail
- center viewport
- right inspector
- bottom status bar

### Phase 3 — state cleanup

Move from “demo properties passed everywhere” to:

- central app state
- explicit renderer state
- narrow UI <-> renderer bridge

### Phase 4 — meaningful viewport features

Depending on the actual product, add:

- camera controls
- scene primitives
- selection outlines
- grid / axes / overlays
- render stats

### Phase 5 — polish

Add:

- typography pass
- iconography
- better spacing system
- hover/pressed states
- shadows / depth cues
- animation where useful

---

## 13. What to keep from the PoC

Keep:

- shared-device Slint + `wgpu` approach
- single-window embedded texture architecture
- native Rust-only stack
- blue retro visual direction
- bounded viewport region inside real app layout

Do not keep as-is:

- theme gallery framing
- ad hoc style duplication
- giant monolithic `src/main.rs`
- demo-only control naming if it does not match the real app
- temporary layout hacks used only to compare styles

---

## 14. Suggested immediate next task

The best next concrete task is:

**Create a fresh “real app shell” screen using the blue retro theme, with no theme tabs at all.**

Specifically:

- top bar
- left tool rail
- center embedded `wgpu` viewport
- right inspector panel
- bottom status strip
- 3-5 realistic controls only

That will answer the real question better than continuing to iterate on a style-gallery demo.

---

## 15. Acceptance criteria for the new app base

The new application base should feel successful when:

- it launches as one coherent native Slint app
- the embedded `wgpu` viewport is clearly part of the UI, not separate
- the blue-retro theme feels calm and intentional
- the inspector/tool chrome does not vertically stretch awkwardly
- the UI still reads as a serious tool, not a toy demo
- the architecture is modular enough to grow into a real app

---

## 16. Short version

If building the real app tomorrow, do this:

1. copy the embedded `wgpu` texture-import architecture
2. throw away the theme gallery
3. keep the **Blue CRT workstation** direction
4. build one realistic app shell around the viewport
5. preserve natural-height control panels
6. modularize Rust state and renderer code early

That is the correct direction.
