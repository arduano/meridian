use std::{
    cell::RefCell,
    env,
    path::PathBuf,
    rc::Rc,
    time::{Duration, Instant},
};

use slint::wgpu_28::wgpu;

use crate::{
    error::MeridianError,
    midi::{MIDIFileBase, MIDIFileUnion},
    render::{
        ProjectedScene, RendererKind, SceneLayout, project_scene_into,
        wgpu::{PrimitiveSceneRenderer, VIEWPORT_FORMAT},
    },
};

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

struct AppModel {
    midi: Option<MIDIFileUnion>,
    midi_path: Option<PathBuf>,
    layout: SceneLayout,
    current_time: f64,
    playing: bool,
    last_tick: Instant,
}

impl AppModel {
    fn new(options: &UiOptions, midi: Option<MIDIFileUnion>) -> Self {
        Self {
            midi,
            midi_path: options.midi_path.clone(),
            layout: SceneLayout {
                renderer: options.renderer,
                view_range: options.view_range,
                first_key: options.first_key,
                last_key: options.last_key,
                viewport_width: 1280,
                viewport_height: 720,
                ..Default::default()
            },
            current_time: options.start_time.max(0.0),
            playing: false,
            last_tick: Instant::now(),
        }
    }

    fn midi_length(&self) -> f64 {
        self.midi
            .as_ref()
            .and_then(|midi| midi.midi_length())
            .unwrap_or(0.0)
    }

    fn total_notes_text(&self) -> String {
        self.midi
            .as_ref()
            .and_then(|midi| midi.stats().total_notes)
            .map(|value| value.to_string())
            .unwrap_or_else(|| "0".into())
    }

    fn status_text(&self) -> String {
        if let Some(path) = &self.midi_path {
            format!(
                "RAM backend loaded. Blue shell active. Rendering from shared note-view abstractions for {}",
                path.display()
            )
        } else {
            "Launch with `meridian ui --midi <file.mid>` to render a MIDI".into()
        }
    }

    fn sync_time(&mut self) {
        let now = Instant::now();
        if self.playing {
            self.current_time += now.duration_since(self.last_tick).as_secs_f64();
            self.current_time = self.current_time.min(self.midi_length().max(0.0));
            if self.current_time >= self.midi_length() && self.midi_length() > 0.0 {
                self.playing = false;
            }
        }
        self.last_tick = now;
    }

    fn step_time(&mut self, delta: f64) {
        self.sync_time();
        self.current_time = (self.current_time + delta).clamp(0.0, self.midi_length().max(0.0));
    }

    fn zoom(&mut self, delta: f64) {
        self.layout.view_range = (self.layout.view_range + delta).clamp(1.0, 30.0);
    }

    fn toggle_play(&mut self) {
        self.sync_time();
        self.playing = !self.playing;
    }
}

struct ViewportTexture {
    texture: wgpu::Texture,
    size: (u32, u32),
}

struct ViewportRenderer {
    app: slint::Weak<App>,
    model: Rc<RefCell<AppModel>>,
    renderer: Option<PrimitiveSceneRenderer>,
    viewport: Option<ViewportTexture>,
    enabled: bool,
    fps_smoothed: f32,
    last_frame_at: Option<Instant>,
    scene: ProjectedScene,
}

impl ViewportRenderer {
    fn new(app: slint::Weak<App>, model: Rc<RefCell<AppModel>>, enabled: bool) -> Self {
        Self {
            app,
            model,
            renderer: None,
            viewport: None,
            enabled,
            fps_smoothed: 0.0,
            last_frame_at: None,
            scene: ProjectedScene::default(),
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

        {
            let mut model = self.model.borrow_mut();
            model.sync_time();

            let midi_path = model
                .midi_path
                .as_ref()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| "No MIDI loaded".into());
            app.set_midi_path_text(midi_path.into());
            app.set_time_text(format!("{:.3} s", model.current_time).into());
            app.set_length_text(format!("{:.3} s", model.midi_length()).into());
            app.set_note_count_text(model.total_notes_text().into());
            app.set_view_range_text(format!("{:.1} s", model.layout.view_range).into());
            app.set_play_label(if model.playing {
                "Pause".into()
            } else {
                "Play".into()
            });

            let current_time = model.current_time;
            model.layout.viewport_width = width;
            model.layout.viewport_height = height;
            let layout = model.layout;

            if let Some(midi) = model.midi.as_mut() {
                project_scene_into(midi, current_time, &layout, &mut self.scene);
                app.set_visible_note_count_text(self.scene.visible_notes.to_string().into());
                app.set_active_keys_text(self.scene.active_keys.to_string().into());
                app.set_status_text(model.status_text().into());
            } else {
                self.scene.clear();
                app.set_visible_note_count_text("0".into());
                app.set_active_keys_text("0".into());
                app.set_status_text(model.status_text().into());
            }
        }

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

        renderer.render(device, queue, &viewport.texture, &self.scene);
    }
}

pub fn run_ui(options: UiOptions) -> Result<(), MeridianError> {
    let backend_selector = slint::BackendSelector::new();
    if options.disable_wgpu {
        backend_selector.select()?;
    } else {
        backend_selector
            .require_wgpu_28(slint::wgpu_28::WGPUConfiguration::default())
            .select()?;
    }

    let midi = if let Some(path) = &options.midi_path {
        Some(MIDIFileUnion::load_ram(path)?)
    } else {
        None
    };
    let model = Rc::new(RefCell::new(AppModel::new(&options, midi)));

    let app = App::new()?;
    {
        let model = model.borrow();
        app.set_midi_path_text(
            model
                .midi_path
                .as_ref()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| "No MIDI loaded".into())
                .into(),
        );
        app.set_status_text(model.status_text().into());
        app.set_length_text(format!("{:.3} s", model.midi_length()).into());
        app.set_note_count_text(model.total_notes_text().into());
        app.set_view_range_text(format!("{:.1} s", model.layout.view_range).into());
    }

    {
        let model = Rc::clone(&model);
        let app_weak = app.as_weak();
        app.on_step_time(move |delta| {
            model.borrow_mut().step_time(delta as f64);
            if let Some(app) = app_weak.upgrade() {
                app.window().request_redraw();
            }
        });
    }
    {
        let model = Rc::clone(&model);
        let app_weak = app.as_weak();
        app.on_zoom(move |delta| {
            model.borrow_mut().zoom(delta as f64);
            if let Some(app) = app_weak.upgrade() {
                app.window().request_redraw();
            }
        });
    }
    {
        let model = Rc::clone(&model);
        let app_weak = app.as_weak();
        app.on_toggle_play(move || {
            model.borrow_mut().toggle_play();
            if let Some(app) = app_weak.upgrade() {
                app.window().request_redraw();
            }
        });
    }

    if !options.disable_wgpu {
        let renderer = Rc::new(RefCell::new(ViewportRenderer::new(
            app.as_weak(),
            Rc::clone(&model),
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
    let model_for_timer = Rc::clone(&model);
    animation_timer.start(
        slint::TimerMode::Repeated,
        Duration::from_millis(16),
        move || {
            if model_for_timer.borrow().playing {
                if let Some(app) = app_for_timer.upgrade() {
                    app.window().request_redraw();
                }
            }
        },
    );

    app.window().request_redraw();
    app.run()?;
    Ok(())
}
