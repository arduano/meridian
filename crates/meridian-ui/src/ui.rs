use std::{
    cell::RefCell,
    env,
    path::PathBuf,
    rc::Rc,
    time::{Duration, Instant},
};

use meridian_core::{
    CoreHandle, MeridianError, RenderedFrame,
    protocol::{CoreCommand, CoreEvent, StateSnapshot},
    render::{
        RendererKind,
        wgpu::{PrimitiveSceneRenderer, VIEWPORT_FORMAT},
    },
    spawn_core,
};
use slint::wgpu_28::wgpu;

slint::slint! {
    component ActionButton inherits Rectangle {
        in property <string> label;
        in property <bool> active: false;
        callback pressed;

        min-width: 70px;
        min-height: 30px;
        border-radius: 7px;
        border-width: 1px;
        border-color: active ? #77a7ff : #44576d;
        background: touch.pressed ? #2a3b52 : active ? #405980 : #1a2331;

        Text {
            text: parent.label;
            color: active ? #f4f7ff : #d2def2;
            horizontal-alignment: center;
            vertical-alignment: center;
            font-family: "monospace";
            font-size: 12px;
            font-weight: 700;
        }

        touch := TouchArea {
            clicked => { root.pressed(); }
        }
    }

    component StatLine inherits Rectangle {
        in property <string> label;
        in property <string> value;

        min-height: 48px;
        border-radius: 10px;
        border-width: 1px;
        border-color: #32455d;
        background: #172334;

        VerticalLayout {
            padding: 8px;
            spacing: 2px;

            Text {
                text: label;
                color: #89a0bf;
                font-family: "monospace";
                font-size: 11px;
            }

            Text {
                text: value;
                color: #edf4ff;
                font-family: "monospace";
                font-size: 13px;
                font-weight: 700;
                overflow: elide;
            }
        }
    }

    export component App inherits Window {
        in-out property <image> viewport-image;
        in-out property <string> midi-path-text: "No MIDI loaded";
        in-out property <string> status-text: "Waiting for MIDI";
        in-out property <string> time-text: "0.000 s";
        in-out property <string> length-text: "0.000 s";
        in-out property <string> note-count-text: "0";
        in-out property <string> visible-note-count-text: "0";
        in-out property <string> active-keys-text: "0";
        in-out property <string> view-range-text: "8.0 s";
        in-out property <string> play-label: "Play";
        in-out property <string> fps-text: "--";

        out property <float> viewport-px-width: viewport-box.width / 1px;
        out property <float> viewport-px-height: viewport-box.height / 1px;

        callback step-time(float);
        callback zoom(float);
        callback toggle-play();

        title: "Meridian";
        preferred-width: 1480px;
        preferred-height: 920px;
        background: rgb(11, 18, 32);

        HorizontalLayout {
            padding: 14px;
            spacing: 12px;

            Rectangle {
                width: 296px;
                border-radius: 18px;
                border-width: 1px;
                border-color: #26354d;
                background: #121b29;

                VerticalLayout {
                    padding: 12px;
                    spacing: 10px;

                    Text {
                        text: "MERIDIAN // BLUE CRT";
                        color: rgb(238, 245, 255);
                        font-family: "monospace";
                        font-size: 18px;
                        font-weight: 700;
                    }

                    Text {
                        text: "Retro workstation shell with a native note viewport embedded into the main panel.";
                        color: #8ca3c5;
                        font-size: 12px;
                        wrap: word-wrap;
                    }

                    Rectangle { height: 1px; background: #223147; }

                    StatLine { label: "MIDI"; value: root.midi-path-text; }
                    StatLine { label: "TIME"; value: root.time-text; }
                    StatLine { label: "LENGTH"; value: root.length-text; }
                    StatLine { label: "FPS"; value: root.fps-text; }
                    StatLine { label: "VISIBLE"; value: root.visible-note-count-text; }
                    StatLine { label: "ACTIVE KEYS"; value: root.active-keys-text; }
                    StatLine { label: "TOTAL NOTES"; value: root.note-count-text; }
                    StatLine { label: "VIEW RANGE"; value: root.view-range-text; }

                    Rectangle {
                        border-radius: 14px;
                        border-width: 1px;
                        border-color: #314760;
                        background: #172130;

                        VerticalLayout {
                            padding: 10px;
                            spacing: 8px;

                            Text {
                                text: "TRANSPORT";
                                color: #87a7d6;
                                font-family: "monospace";
                                font-size: 12px;
                            }

                            HorizontalLayout {
                                spacing: 6px;
                                ActionButton { label: "-5s"; pressed => { root.step-time(-5.0); } }
                                ActionButton { label: "-1s"; pressed => { root.step-time(-1.0); } }
                                ActionButton { label: "+1s"; pressed => { root.step-time(1.0); } }
                                ActionButton { label: "+5s"; pressed => { root.step-time(5.0); } }
                            }

                            HorizontalLayout {
                                spacing: 6px;
                                ActionButton { label: root.play-label; active: root.play-label == "Pause"; pressed => { root.toggle-play(); } }
                                ActionButton { label: "Zoom-"; pressed => { root.zoom(-1.0); } }
                                ActionButton { label: "Zoom+"; pressed => { root.zoom(1.0); } }
                            }
                        }
                    }

                    Rectangle {
                        border-radius: 14px;
                        border-width: 1px;
                        border-color: #2d4360;
                        background: #15202f;

                        VerticalLayout {
                            padding: 10px;
                            spacing: 4px;

                            Text {
                                text: "STATUS";
                                color: #87a7d6;
                                font-family: "monospace";
                                font-size: 12px;
                            }

                            Text {
                                text: root.status-text;
                                color: rgb(217, 232, 255);
                                font-family: "monospace";
                                font-size: 12px;
                                wrap: word-wrap;
                            }
                        }
                    }

                    Rectangle {
                        vertical-stretch: 1;
                        border-radius: 14px;
                        background: #101926;
                        border-width: 1px;
                        border-color: #203046;
                    }
                }
            }

            Rectangle {
                border-radius: 18px;
                border-width: 1px;
                border-color: #26354d;
                background: #101826;
                horizontal-stretch: 1;
                vertical-stretch: 1;

                VerticalLayout {
                    padding: 10px;
                    spacing: 8px;

                    Rectangle {
                        border-radius: 12px;
                        border-width: 1px;
                        border-color: #31455f;
                        background: #162131;

                        HorizontalLayout {
                            padding: 10px;
                            spacing: 8px;

                            Text {
                                text: "Viewport";
                                color: #f1f6ff;
                                font-size: 16px;
                                font-weight: 700;
                            }

                            Rectangle { horizontal-stretch: 1; background: #00000000; }

                            Rectangle {
                                border-radius: 999px;
                                border-width: 1px;
                                border-color: #496584;
                                background: #23344a;
                                min-width: 112px;
                                min-height: 26px;

                                Text {
                                    text: "CRT / NOTE FIELD";
                                    color: #c7dbfb;
                                    horizontal-alignment: center;
                                    vertical-alignment: center;
                                    font-family: "monospace";
                                    font-size: 11px;
                                }
                            }
                        }
                    }

                    viewport-box := Rectangle {
                        border-radius: 16px;
                        border-width: 1px;
                        border-color: rgb(52, 81, 110);
                        background: rgb(13, 22, 36);
                        horizontal-stretch: 1;
                        vertical-stretch: 1;

                        Rectangle {
                            x: 18px;
                            y: 18px;
                            width: parent.width - 36px;
                            height: parent.height - 36px;
                            border-radius: 12px;
                            border-width: 1px;
                            border-color: #456789;
                            background: rgb(7, 17, 29);

                            Image {
                                source: root.viewport-image;
                                width: parent.width;
                                height: parent.height;
                                image-fit: fill;
                            }
                        }
                    }
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct UiOptions {
    pub midi_path: Option<PathBuf>,
    pub renderer: RendererKind,
    pub start_time: f64,
    pub view_range: f64,
    pub first_key: u8,
    pub last_key: u8,
    pub disable_wgpu: bool,
}

impl Default for UiOptions {
    fn default() -> Self {
        Self {
            midi_path: None,
            renderer: RendererKind::Pfa,
            start_time: 0.0,
            view_range: 8.0,
            first_key: 0,
            last_key: 127,
            disable_wgpu: matches!(
                env::var("MERIDIAN_DISABLE_WGPU").as_deref(),
                Ok("1" | "true" | "yes")
            ),
        }
    }
}

struct ViewportTexture {
    texture: wgpu::Texture,
    size: (u32, u32),
}

struct ViewportRenderer {
    app: slint::Weak<App>,
    core: CoreHandle,
    shared_state: Rc<RefCell<Option<StateSnapshot>>>,
    renderer: Option<PrimitiveSceneRenderer>,
    viewport: Option<ViewportTexture>,
    enabled: bool,
    fps_smoothed: f32,
    last_frame_at: Option<Instant>,
}

impl ViewportRenderer {
    fn new(
        app: slint::Weak<App>,
        core: CoreHandle,
        shared_state: Rc<RefCell<Option<StateSnapshot>>>,
        enabled: bool,
    ) -> Self {
        Self {
            app,
            core,
            shared_state,
            renderer: None,
            viewport: None,
            enabled,
            fps_smoothed: 0.0,
            last_frame_at: None,
        }
    }

    fn handle(&mut self, state: slint::RenderingState, graphics_api: &slint::GraphicsAPI<'_>) {
        if !self.enabled {
            if let Some(app) = self.app.upgrade() {
                app.set_status_text(
                    "Accelerated viewport disabled via MERIDIAN_DISABLE_WGPU=1".into(),
                );
            }
            return;
        }

        match (state, graphics_api) {
            (slint::RenderingState::RenderingSetup, slint::GraphicsAPI::WGPU28 { device, .. }) => {
                if self.renderer.is_none() {
                    self.renderer = Some(PrimitiveSceneRenderer::new(device));
                }
            }
            (
                slint::RenderingState::BeforeRendering,
                slint::GraphicsAPI::WGPU28 { device, queue, .. },
            ) => self.render(device, queue),
            (slint::RenderingState::RenderingTeardown, _) => {
                self.renderer = None;
                self.viewport = None;
            }
            _ => {}
        }
    }

    fn render(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        let Some(app) = self.app.upgrade() else {
            return;
        };

        let width = app.get_viewport_px_width().max(1.0) as u32;
        let height = app.get_viewport_px_height().max(1.0) as u32;

        if self
            .viewport
            .as_ref()
            .is_none_or(|viewport| viewport.size != (width, height))
        {
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("MeridianViewportTexture"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: VIEWPORT_FORMAT,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });

            let imported_image = slint::Image::try_from(texture.clone())
                .expect("Slint should accept the off-screen viewport texture");

            self.viewport = Some(ViewportTexture {
                texture,
                size: (width, height),
            });
            app.set_viewport_image(imported_image);
        }

        let Some(renderer) = self.renderer.as_mut() else {
            return;
        };
        let Some(viewport) = self.viewport.as_ref() else {
            return;
        };

        let Ok(frame) = self.core.render_frame(Some(width), Some(height)) else {
            app.set_status_text("Core render request failed".into());
            return;
        };
        apply_rendered_frame_to_app(&app, &self.shared_state, &frame);

        let now = Instant::now();
        if let Some(previous) = self.last_frame_at {
            let delta = now.duration_since(previous).as_secs_f32();
            if delta > 0.0 {
                let fps = 1.0 / delta;
                self.fps_smoothed = if self.fps_smoothed == 0.0 {
                    fps
                } else {
                    self.fps_smoothed * 0.88 + fps * 0.12
                };
                app.set_fps_text(format!("{:.1}", self.fps_smoothed).into());
            }
        }
        self.last_frame_at = Some(now);

        renderer.render(device, queue, &viewport.texture, &frame.scene);
    }
}

pub fn run_ui(options: UiOptions) -> Result<(), MeridianError> {
    let backend_selector = slint::BackendSelector::new();
    if options.disable_wgpu {
        backend_selector
            .select()
            .map_err(|e| MeridianError::Platform(e.to_string()))?;
    } else {
        backend_selector
            .require_wgpu_28(slint::wgpu_28::WGPUConfiguration::default())
            .select()
            .map_err(|e| MeridianError::Platform(e.to_string()))?;
    }

    let app = App::new().map_err(|e| MeridianError::Platform(e.to_string()))?;
    let core = spawn_core();
    let shared_state = Rc::new(RefCell::new(None));
    initialize_core(&core, &options, &app, &shared_state)?;

    {
        let core = core.clone();
        let app_weak = app.as_weak();
        let shared_state = Rc::clone(&shared_state);
        app.on_step_time(move |delta| {
            if let Some(app) = app_weak.upgrade() {
                if let Ok(events) = core.request(CoreCommand::StepTime {
                    delta: delta as f64,
                }) {
                    apply_events_to_app(&app, &shared_state, &events);
                }
                app.window().request_redraw();
            }
        });
    }
    {
        let core = core.clone();
        let app_weak = app.as_weak();
        let shared_state = Rc::clone(&shared_state);
        app.on_zoom(move |delta| {
            if let Some(app) = app_weak.upgrade() {
                let current_view_range = shared_state
                    .borrow()
                    .as_ref()
                    .map(|snapshot| snapshot.view_range)
                    .unwrap_or(8.0);
                if let Ok(events) = core.request(CoreCommand::SetLayout {
                    renderer: None,
                    view_range: Some((current_view_range + delta as f64).clamp(1.0, 30.0)),
                    first_key: None,
                    last_key: None,
                    viewport_width: None,
                    viewport_height: None,
                }) {
                    apply_events_to_app(&app, &shared_state, &events);
                }
                app.window().request_redraw();
            }
        });
    }
    {
        let core = core.clone();
        let app_weak = app.as_weak();
        let shared_state = Rc::clone(&shared_state);
        app.on_toggle_play(move || {
            if let Some(app) = app_weak.upgrade() {
                if let Ok(events) = core.request(CoreCommand::TogglePlaying) {
                    apply_events_to_app(&app, &shared_state, &events);
                }
                app.window().request_redraw();
            }
        });
    }

    if !options.disable_wgpu {
        let renderer = Rc::new(RefCell::new(ViewportRenderer::new(
            app.as_weak(),
            core.clone(),
            Rc::clone(&shared_state),
            true,
        )));
        let renderer_for_notifier = Rc::clone(&renderer);
        app.window()
            .set_rendering_notifier(move |state, graphics_api| {
                renderer_for_notifier
                    .borrow_mut()
                    .handle(state, graphics_api);
            })
            .map_err(|e| MeridianError::SlintNotifier(e.to_string()))?;
    } else {
        app.set_status_text("Accelerated viewport disabled via MERIDIAN_DISABLE_WGPU=1".into());
    }

    let animation_timer = slint::Timer::default();
    let app_for_timer = app.as_weak();
    let core_for_timer = core.clone();
    let shared_state_for_timer = Rc::clone(&shared_state);
    let disable_wgpu = options.disable_wgpu;
    animation_timer.start(
        slint::TimerMode::Repeated,
        Duration::from_millis(16),
        move || {
            if let Some(app) = app_for_timer.upgrade() {
                if let Ok(events) = core_for_timer.request(CoreCommand::GetState) {
                    let playing = events.iter().find_map(|event| match event {
                        CoreEvent::StateSnapshot { state } => Some(state.playing),
                        _ => None,
                    });
                    apply_events_to_app(&app, &shared_state_for_timer, &events);
                    if playing == Some(true) || disable_wgpu {
                        app.window().request_redraw();
                    }
                }
                if disable_wgpu {
                    app.window().request_redraw();
                } else if app.get_play_label() == "Pause" {
                    app.window().request_redraw();
                }
            }
        },
    );

    app.window().request_redraw();
    app.run()
        .map_err(|e| MeridianError::Platform(e.to_string()))?;
    Ok(())
}

fn initialize_core(
    core: &CoreHandle,
    options: &UiOptions,
    app: &App,
    shared_state: &Rc<RefCell<Option<StateSnapshot>>>,
) -> Result<(), MeridianError> {
    let events = core.request(CoreCommand::SetLayout {
        renderer: Some(options.renderer),
        view_range: Some(options.view_range),
        first_key: Some(options.first_key),
        last_key: Some(options.last_key),
        viewport_width: Some(1280),
        viewport_height: Some(720),
    })?;
    apply_events_to_app(app, shared_state, &events);
    let events = core.request(CoreCommand::SetTime {
        time: options.start_time.max(0.0),
    })?;
    apply_events_to_app(app, shared_state, &events);
    if let Some(path) = &options.midi_path {
        let events = core.request(CoreCommand::LoadMidi { path: path.clone() })?;
        apply_events_to_app(app, shared_state, &events);
    }
    Ok(())
}

fn apply_events_to_app(
    app: &App,
    shared_state: &Rc<RefCell<Option<StateSnapshot>>>,
    events: &[CoreEvent],
) {
    for event in events {
        apply_event_to_app(app, shared_state, event);
    }
}

fn apply_event_to_app(
    app: &App,
    shared_state: &Rc<RefCell<Option<StateSnapshot>>>,
    event: &CoreEvent,
) {
    match event {
        CoreEvent::StateSnapshot { state } | CoreEvent::MidiLoaded { state, .. } => {
            apply_state_to_app(app, shared_state, state);
            app.set_status_text(status_text(state).into());
            app.set_visible_note_count_text("0".into());
            app.set_active_keys_text("0".into());
        }
        CoreEvent::FrameProjected { state, stats, .. } => {
            apply_state_to_app(app, shared_state, state);
            app.set_visible_note_count_text(stats.visible_notes.to_string().into());
            app.set_active_keys_text(stats.active_keys.to_string().into());
            app.set_status_text(status_text(state).into());
        }
        CoreEvent::FrameSaved { state, output, .. } => {
            apply_state_to_app(app, shared_state, state);
            app.set_status_text(format!("Saved frame to {}", output.display()).into());
        }
        CoreEvent::Error { message, .. } => {
            app.set_status_text(message.clone().into());
        }
        CoreEvent::ShutdownComplete => {}
    }
}

fn apply_state_to_app(
    app: &App,
    shared_state: &Rc<RefCell<Option<StateSnapshot>>>,
    state: &StateSnapshot,
) {
    *shared_state.borrow_mut() = Some(state.clone());
    app.set_midi_path_text(
        state
            .midi_path
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "No MIDI loaded".into())
            .into(),
    );
    app.set_time_text(format!("{:.3} s", state.current_time).into());
    app.set_length_text(format!("{:.3} s", state.midi_length).into());
    app.set_note_count_text(state.total_notes.to_string().into());
    app.set_view_range_text(format!("{:.1} s", state.view_range).into());
    app.set_play_label(if state.playing {
        "Pause".into()
    } else {
        "Play".into()
    });
}

fn apply_rendered_frame_to_app(
    app: &App,
    shared_state: &Rc<RefCell<Option<StateSnapshot>>>,
    frame: &RenderedFrame,
) {
    apply_state_to_app(app, shared_state, &frame.state);
    app.set_visible_note_count_text(frame.stats.visible_notes.to_string().into());
    app.set_active_keys_text(frame.stats.active_keys.to_string().into());
    app.set_status_text(status_text(&frame.state).into());
}

fn status_text(state: &StateSnapshot) -> String {
    if let Some(path) = &state.midi_path {
        format!(
            "Core session active. UI frontend attached. Rendering {} through the shared event bus.",
            path.display()
        )
    } else {
        "Launch with `meridian-ui --midi <file.mid>` to render a MIDI".into()
    }
}
