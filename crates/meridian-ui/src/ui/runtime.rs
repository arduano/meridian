use std::{
    cell::RefCell,
    rc::Rc,
    sync::{Arc, Mutex},
    time::Duration,
};

use meridian_core::{CoreHandle, MeridianError, render::RendererKind, spawn_core};
use slint::ComponentHandle;

use super::{
    core_bridge::UiCoreBridge,
    state::{UiOptions, apply_events_to_app},
    view::App,
    view_model::UiViewModel,
    viewport::ViewportRenderer,
};

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
    let bridge = UiCoreBridge::new(spawn_core());
    let shared_state = Arc::new(Mutex::new(UiViewModel::default()));
    let pending_viewport_image = Rc::new(RefCell::new(None));
    let viewport_size = Rc::new(RefCell::new((1280_u32, 720_u32)));
    initialize_core(&bridge, &options, &app, &shared_state)?;
    wire_callbacks(&app, &bridge, &shared_state);
    install_viewport(
        &app,
        bridge.core(),
        &shared_state,
        &pending_viewport_image,
        &viewport_size,
        options.disable_wgpu,
    )?;
    let _animation_timer = install_timer(
        &app,
        &bridge,
        &shared_state,
        &pending_viewport_image,
        &viewport_size,
        options.disable_wgpu,
    );

    app.window().request_redraw();
    app.run()
        .map_err(|e| MeridianError::Platform(e.to_string()))
}

pub(crate) fn initialize_core(
    bridge: &UiCoreBridge,
    options: &UiOptions,
    app: &App,
    shared_state: &Arc<Mutex<UiViewModel>>,
) -> Result<(), MeridianError> {
    bridge.initialize(options, shared_state)?;
    let initial = bridge.refresh_state(shared_state)?;
    apply_events_to_app(app, shared_state, &initial);
    Ok(())
}

fn wire_callbacks(app: &App, bridge: &UiCoreBridge, shared_state: &Arc<Mutex<UiViewModel>>) {
    {
        let bridge = bridge.clone();
        let app_weak = app.as_weak();
        let shared_state = Arc::clone(shared_state);
        app.on_step_time(move |delta| {
            if let Some(app) = app_weak.upgrade() {
                if let Ok(events) = bridge.step_time(delta as f64, &shared_state) {
                    apply_events_to_app(&app, &shared_state, &events);
                }
                app.window().request_redraw();
            }
        });
    }
    {
        let bridge = bridge.clone();
        let app_weak = app.as_weak();
        let shared_state = Arc::clone(shared_state);
        app.on_zoom(move |delta| {
            if let Some(app) = app_weak.upgrade() {
                if let Ok(events) = bridge.zoom(delta as f64, &shared_state) {
                    apply_events_to_app(&app, &shared_state, &events);
                }
                app.window().request_redraw();
            }
        });
    }
    {
        let bridge = bridge.clone();
        let app_weak = app.as_weak();
        let shared_state = Arc::clone(shared_state);
        app.on_toggle_play(move || {
            if let Some(app) = app_weak.upgrade() {
                if let Ok(events) = bridge.toggle_play(&shared_state) {
                    apply_events_to_app(&app, &shared_state, &events);
                }
                app.window().request_redraw();
            }
        });
    }
    {
        let bridge = bridge.clone();
        let app_weak = app.as_weak();
        let shared_state = Arc::clone(shared_state);
        app.on_seek_time(move |time| {
            if let Some(app) = app_weak.upgrade() {
                if let Ok(events) = bridge.seek_time(time as f64, &shared_state) {
                    apply_events_to_app(&app, &shared_state, &events);
                }
                app.window().request_redraw();
            }
        });
    }
    {
        let bridge = bridge.clone();
        let app_weak = app.as_weak();
        let shared_state = Arc::clone(shared_state);
        app.on_select_renderer(move |renderer| {
            if let Some(app) = app_weak.upgrade() {
                let renderer = match renderer.as_str() {
                    "flat" => RendererKind::Flat,
                    "piano_trail_classic" | "3d" => RendererKind::PianoTrailClassic,
                    _ => RendererKind::Pfa,
                };
                if let Ok(events) = bridge.set_renderer(renderer, &shared_state) {
                    apply_events_to_app(&app, &shared_state, &events);
                }
                app.window().request_redraw();
            }
        });
    }
}

fn install_viewport(
    app: &App,
    core: &CoreHandle,
    shared_state: &Arc<Mutex<UiViewModel>>,
    pending_viewport_image: &Rc<RefCell<Option<slint::Image>>>,
    viewport_size: &Rc<RefCell<(u32, u32)>>,
    disable_wgpu: bool,
) -> Result<(), MeridianError> {
    if disable_wgpu {
        app.set_status_text("Accelerated viewport disabled via MERIDIAN_DISABLE_WGPU=1".into());
        return Ok(());
    }

    let renderer = std::rc::Rc::new(std::cell::RefCell::new(ViewportRenderer::new(
        app.as_weak(),
        core.clone(),
        Arc::clone(shared_state),
        Rc::clone(pending_viewport_image),
        Rc::clone(viewport_size),
        true,
    )));
    let renderer_for_notifier = std::rc::Rc::clone(&renderer);
    app.window()
        .set_rendering_notifier(move |state, graphics_api| {
            renderer_for_notifier
                .borrow_mut()
                .handle(state, graphics_api);
        })
        .map_err(|e| MeridianError::SlintNotifier(e.to_string()))
}

fn install_timer(
    app: &App,
    bridge: &UiCoreBridge,
    shared_state: &Arc<Mutex<UiViewModel>>,
    pending_viewport_image: &Rc<RefCell<Option<slint::Image>>>,
    viewport_size: &Rc<RefCell<(u32, u32)>>,
    disable_wgpu: bool,
) -> slint::Timer {
    let animation_timer = slint::Timer::default();
    let app_for_timer = app.as_weak();
    let bridge_for_timer = bridge.clone();
    let shared_state_for_timer = Arc::clone(shared_state);
    let pending_viewport_image_for_timer = Rc::clone(pending_viewport_image);
    let viewport_size_for_timer = Rc::clone(viewport_size);

    animation_timer.start(
        slint::TimerMode::Repeated,
        Duration::from_millis(16),
        move || {
            if let Some(app) = app_for_timer.upgrade() {
                *viewport_size_for_timer.borrow_mut() = (
                    app.get_viewport_px_width().max(1.0) as u32,
                    app.get_viewport_px_height().max(1.0) as u32,
                );
                if let Some(image) = pending_viewport_image_for_timer.borrow_mut().take() {
                    app.set_viewport_image(image);
                }
                if let Ok(events) = bridge_for_timer.refresh_state(&shared_state_for_timer) {
                    apply_events_to_app(&app, &shared_state_for_timer, &events);
                    let playing = shared_state_for_timer
                        .lock()
                        .expect("shared UI state mutex poisoned")
                        .transport
                        .playing;
                    if playing || disable_wgpu {
                        app.window().request_redraw();
                    }
                }
                if disable_wgpu || app.get_play_label() == "Pause" {
                    app.window().request_redraw();
                }
            }
        },
    );
    animation_timer
}
