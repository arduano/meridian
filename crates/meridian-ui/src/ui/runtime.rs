use std::{
    cell::RefCell,
    rc::Rc,
    sync::{Arc, Mutex},
    time::Duration,
};

use meridian_core::{
    CoreHandle, MeridianError,
    protocol::{CoreCommand, CoreEvent, StateSnapshot},
    render::SceneLayout,
    spawn_core,
};
use slint::ComponentHandle;

use super::{
    state::{UiOptions, apply_events_to_app},
    view::App,
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
    let core = spawn_core();
    let shared_state = Arc::new(Mutex::new(None));
    let pending_viewport_image = Rc::new(RefCell::new(None));
    let viewport_size = Rc::new(RefCell::new((1280_u32, 720_u32)));
    initialize_core(&core, &options, &app, &shared_state)?;
    wire_callbacks(&app, &core, &shared_state);
    install_viewport(
        &app,
        &core,
        &shared_state,
        &pending_viewport_image,
        &viewport_size,
        options.disable_wgpu,
    )?;
    let _animation_timer = install_timer(
        &app,
        &core,
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
    core: &CoreHandle,
    options: &UiOptions,
    app: &App,
    shared_state: &Arc<Mutex<Option<StateSnapshot>>>,
) -> Result<(), MeridianError> {
    let mut layout = SceneLayout::default();
    layout.set_renderer_kind(options.renderer);
    for events in [
        core.request(CoreCommand::SetSceneConfig {
            scene: layout.scene.clone(),
        })?,
        core.request(CoreCommand::SetViewRange {
            seconds: options.view_range,
        })?,
        core.request(CoreCommand::SetKeyRange {
            first_key: options.first_key,
            last_key: options.last_key,
        })?,
        core.request(CoreCommand::SetViewport {
            width: 1280,
            height: 720,
        })?,
        core.request(CoreCommand::SetTime {
            time: options.start_time.max(0.0),
        })?,
    ] {
        apply_events_to_app(app, shared_state, &events);
    }
    if let Some(path) = &options.midi_path {
        let events = core.request(CoreCommand::LoadMidi { path: path.clone() })?;
        apply_events_to_app(app, shared_state, &events);
    }
    Ok(())
}

fn wire_callbacks(app: &App, core: &CoreHandle, shared_state: &Arc<Mutex<Option<StateSnapshot>>>) {
    {
        let core = core.clone();
        let app_weak = app.as_weak();
        let shared_state = Arc::clone(shared_state);
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
        let shared_state = Arc::clone(shared_state);
        app.on_zoom(move |delta| {
            if let Some(app) = app_weak.upgrade() {
                let current_view_range = shared_state
                    .lock()
                    .expect("shared UI state mutex poisoned")
                    .as_ref()
                    .map(|s| s.view_range)
                    .unwrap_or(8.0);
                if let Ok(events) = core.request(CoreCommand::SetViewRange {
                    seconds: (current_view_range + delta as f64).clamp(1.0, 30.0),
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
        let shared_state = Arc::clone(shared_state);
        app.on_toggle_play(move || {
            if let Some(app) = app_weak.upgrade() {
                if let Ok(events) = core.request(CoreCommand::TogglePlaying) {
                    apply_events_to_app(&app, &shared_state, &events);
                }
                app.window().request_redraw();
            }
        });
    }
    {
        let core = core.clone();
        let app_weak = app.as_weak();
        let shared_state = Arc::clone(shared_state);
        app.on_seek_time(move |time| {
            if let Some(app) = app_weak.upgrade() {
                if let Ok(events) = core.request(CoreCommand::SetTime { time: time as f64 }) {
                    apply_events_to_app(&app, &shared_state, &events);
                }
                app.window().request_redraw();
            }
        });
    }
    {
        let core = core.clone();
        let app_weak = app.as_weak();
        let shared_state = Arc::clone(shared_state);
        app.on_select_renderer(move |renderer| {
            if let Some(app) = app_weak.upgrade() {
                let mut layout = SceneLayout::default();
                match renderer.as_str() {
                    "flat" => layout.set_renderer_kind(meridian_core::render::RendererKind::Flat),
                    _ => layout.set_renderer_kind(meridian_core::render::RendererKind::Pfa),
                }
                if let Ok(events) = core.request(CoreCommand::SetSceneConfig {
                    scene: layout.scene,
                }) {
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
    shared_state: &Arc<Mutex<Option<StateSnapshot>>>,
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
    core: &CoreHandle,
    shared_state: &Arc<Mutex<Option<StateSnapshot>>>,
    pending_viewport_image: &Rc<RefCell<Option<slint::Image>>>,
    viewport_size: &Rc<RefCell<(u32, u32)>>,
    disable_wgpu: bool,
) -> slint::Timer {
    let animation_timer = slint::Timer::default();
    let app_for_timer = app.as_weak();
    let core_for_timer = core.clone();
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
                if disable_wgpu || app.get_play_label() == "Pause" {
                    app.window().request_redraw();
                }
            }
        },
    );
    animation_timer
}
