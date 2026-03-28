use std::{cell::RefCell, rc::Rc, time::Duration};

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
    let shared_state = Rc::new(RefCell::new(None));
    initialize_core(&core, &options, &app, &shared_state)?;
    wire_callbacks(&app, &core, &shared_state);
    install_viewport(&app, &core, &shared_state, options.disable_wgpu)?;
    install_timer(&app, &core, &shared_state, options.disable_wgpu);

    app.window().request_redraw();
    app.run()
        .map_err(|e| MeridianError::Platform(e.to_string()))
}

fn initialize_core(
    core: &CoreHandle,
    options: &UiOptions,
    app: &App,
    shared_state: &Rc<RefCell<Option<StateSnapshot>>>,
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

fn wire_callbacks(app: &App, core: &CoreHandle, shared_state: &Rc<RefCell<Option<StateSnapshot>>>) {
    {
        let core = core.clone();
        let app_weak = app.as_weak();
        let shared_state = Rc::clone(shared_state);
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
        let shared_state = Rc::clone(shared_state);
        app.on_zoom(move |delta| {
            if let Some(app) = app_weak.upgrade() {
                let current_view_range = shared_state
                    .borrow()
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
        let shared_state = Rc::clone(shared_state);
        app.on_toggle_play(move || {
            if let Some(app) = app_weak.upgrade() {
                if let Ok(events) = core.request(CoreCommand::TogglePlaying) {
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
    shared_state: &Rc<RefCell<Option<StateSnapshot>>>,
    disable_wgpu: bool,
) -> Result<(), MeridianError> {
    if disable_wgpu {
        app.set_status_text("Accelerated viewport disabled via MERIDIAN_DISABLE_WGPU=1".into());
        return Ok(());
    }

    let renderer = Rc::new(RefCell::new(ViewportRenderer::new(
        app.as_weak(),
        core.clone(),
        Rc::clone(shared_state),
        true,
    )));
    let renderer_for_notifier = Rc::clone(&renderer);
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
    shared_state: &Rc<RefCell<Option<StateSnapshot>>>,
    disable_wgpu: bool,
) {
    let animation_timer = slint::Timer::default();
    let app_for_timer = app.as_weak();
    let core_for_timer = core.clone();
    let shared_state_for_timer = Rc::clone(shared_state);

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
                if disable_wgpu || app.get_play_label() == "Pause" {
                    app.window().request_redraw();
                }
            }
        },
    );
}
