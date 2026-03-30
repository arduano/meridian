use std::{
    cell::RefCell,
    path::PathBuf,
    rc::Rc,
    sync::{Arc, Mutex},
    time::Duration,
};

use meridian_core::{
    CoreHandle, MeridianError,
    render::{DisplayTimeSpace, RendererKind},
    spawn_core,
};
use slint::ComponentHandle;
use slint::winit_030::{EventResult, WinitWindowAccessor, winit};

use super::{
    core_bridge::UiCoreBridge,
    state::{UiOptions, apply_events_to_app},
    view::{App, MidiLoadState},
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
    install_drag_drop(&app);
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
    wire_transport_callbacks(app, bridge, shared_state);
    wire_midi_callbacks(app, bridge, shared_state);
}

fn wire_transport_callbacks(
    app: &App,
    bridge: &UiCoreBridge,
    shared_state: &Arc<Mutex<UiViewModel>>,
) {
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
    {
        let bridge = bridge.clone();
        let app_weak = app.as_weak();
        let shared_state = Arc::clone(shared_state);
        app.on_select_time_space(move |time_space| {
            if let Some(app) = app_weak.upgrade() {
                let time_space = match time_space.as_str() {
                    "tick" => DisplayTimeSpace::Tick,
                    _ => DisplayTimeSpace::Time,
                };
                if let Ok(events) = bridge.set_time_space(time_space, &shared_state) {
                    apply_events_to_app(&app, &shared_state, &events);
                }
                app.window().request_redraw();
            }
        });
    }
}

/// Wire MIDI file selection, loading, unloading, and drag-drop callbacks.
fn wire_midi_callbacks(app: &App, bridge: &UiCoreBridge, shared_state: &Arc<Mutex<UiViewModel>>) {
    // ── Browse button (open native file dialog) ──
    {
        let app_weak = app.as_weak();
        app.on_select_midi_file(move || {
            let app_weak = app_weak.clone();
            // rfd's async dialog won't block the event loop on supported platforms
            std::thread::spawn(move || {
                let file = rfd::FileDialog::new()
                    .add_filter("MIDI files", &["mid", "midi", "MID", "MIDI"])
                    .add_filter("All files", &["*"])
                    .pick_file();
                if let Some(path) = file {
                    let name: slint::SharedString = path.display().to_string().into();
                    let _ = app_weak.upgrade_in_event_loop(move |app| {
                        app.set_selected_midi_name(name);
                        app.set_render_load_state(MidiLoadState::Selected);
                        app.set_analysis_load_state(MidiLoadState::Selected);
                        app.set_load_error(Default::default());
                    });
                }
            });
        });
    }

    // ── File dropped from OS ──
    {
        let app_weak = app.as_weak();
        let bridge = bridge.clone();
        let shared_state = Arc::clone(shared_state);
        app.on_drop_midi_file(move |path| {
            if let Some(app) = app_weak.upgrade() {
                app.set_selected_midi_name(path.clone());
                app.set_render_load_state(MidiLoadState::Selected);
                app.set_analysis_load_state(MidiLoadState::Selected);
                app.set_load_error(Default::default());
                // Auto-load for the current profile context
                let profile = app.get_active_profile();
                if profile == 0 || profile == 1 || profile == 2 {
                    load_midi_for_render(&app, &bridge, &shared_state, &path);
                } else {
                    load_midi_for_render(&app, &bridge, &shared_state, &path);
                }
            }
        });
    }

    // ── Load for render / preview / audio ──
    {
        let app_weak = app.as_weak();
        let bridge = bridge.clone();
        let shared_state = Arc::clone(shared_state);
        app.on_load_for_render(move || {
            if let Some(app) = app_weak.upgrade() {
                let midi_name = app.get_selected_midi_name();
                if !midi_name.is_empty() {
                    load_midi_for_render(&app, &bridge, &shared_state, &midi_name);
                }
            }
        });
    }

    // ── Load for analysis / modify ──
    {
        let app_weak = app.as_weak();
        let bridge = bridge.clone();
        let shared_state = Arc::clone(shared_state);
        app.on_load_for_analysis(move || {
            if let Some(app) = app_weak.upgrade() {
                let midi_name = app.get_selected_midi_name();
                if !midi_name.is_empty() {
                    load_midi_for_render(&app, &bridge, &shared_state, &midi_name);
                }
            }
        });
    }

    // ── Cancel load ──
    {
        let app_weak = app.as_weak();
        app.on_cancel_load(move || {
            if let Some(app) = app_weak.upgrade() {
                // Revert to "selected" state (we don't have a real cancel mechanism in
                // the core yet, but the UI state should reflect the intent)
                if app.get_render_load_state() == MidiLoadState::Loading {
                    app.set_render_load_state(MidiLoadState::Selected);
                }
                if app.get_analysis_load_state() == MidiLoadState::Loading {
                    app.set_analysis_load_state(MidiLoadState::Selected);
                }
                app.set_loading_progress(0.0);
                app.set_loading_status(Default::default());
            }
        });
    }

    // ── Unload render ──
    {
        let app_weak = app.as_weak();
        app.on_unload_render(move || {
            if let Some(app) = app_weak.upgrade() {
                app.set_render_load_state(MidiLoadState::NoMidi);
                app.set_selected_midi_name(Default::default());
                app.set_loading_progress(0.0);
                app.set_loading_status(Default::default());
            }
        });
    }

    // ── Unload analysis ──
    {
        let app_weak = app.as_weak();
        app.on_unload_analysis(move || {
            if let Some(app) = app_weak.upgrade() {
                app.set_analysis_load_state(MidiLoadState::NoMidi);
                app.set_selected_midi_name(Default::default());
                app.set_loading_progress(0.0);
                app.set_loading_status(Default::default());
            }
        });
    }

    // ── Retry load render ──
    {
        let app_weak = app.as_weak();
        let bridge = bridge.clone();
        let shared_state = Arc::clone(shared_state);
        app.on_retry_load_render(move || {
            if let Some(app) = app_weak.upgrade() {
                let midi_name = app.get_selected_midi_name();
                if !midi_name.is_empty() {
                    load_midi_for_render(&app, &bridge, &shared_state, &midi_name);
                }
            }
        });
    }

    // ── Retry load analysis ──
    {
        let app_weak = app.as_weak();
        let bridge = bridge.clone();
        let shared_state = Arc::clone(shared_state);
        app.on_retry_load_analysis(move || {
            if let Some(app) = app_weak.upgrade() {
                let midi_name = app.get_selected_midi_name();
                if !midi_name.is_empty() {
                    load_midi_for_render(&app, &bridge, &shared_state, &midi_name);
                }
            }
        });
    }
}

/// Actually load a MIDI file through the core bridge and update UI state.
fn load_midi_for_render(
    app: &App,
    bridge: &UiCoreBridge,
    shared_state: &Arc<Mutex<UiViewModel>>,
    midi_path: &str,
) {
    app.set_render_load_state(MidiLoadState::Loading);
    app.set_analysis_load_state(MidiLoadState::Loading);
    app.set_loading_status("Loading MIDI…".into());
    app.set_loading_progress(0.0);
    app.set_load_error(Default::default());

    let path = PathBuf::from(midi_path.to_string());
    match bridge.load_midi(path, shared_state) {
        Ok(events) => {
            apply_events_to_app(app, shared_state, &events);
            app.set_render_load_state(MidiLoadState::Loaded);
            app.set_analysis_load_state(MidiLoadState::Loaded);
            app.set_loading_progress(1.0);
            app.set_loading_status("Loaded".into());
        }
        Err(e) => {
            app.set_render_load_state(MidiLoadState::Error);
            app.set_analysis_load_state(MidiLoadState::Error);
            app.set_load_error(e.to_string().into());
            app.set_loading_status(Default::default());
        }
    }
    app.window().request_redraw();
}

/// Install the winit window event handler for OS file drag-and-drop.
fn install_drag_drop(app: &App) {
    let app_weak = app.as_weak();
    app.window()
        .on_winit_window_event(move |_slint_window, event| match event {
            winit::event::WindowEvent::HoveredFile(_path) => {
                if let Some(app) = app_weak.upgrade() {
                    app.set_drop_hovering(true);
                    app.window().request_redraw();
                }
                EventResult::Propagate
            }
            winit::event::WindowEvent::DroppedFile(path) => {
                if let Some(app) = app_weak.upgrade() {
                    app.set_drop_hovering(false);
                    let path_str: slint::SharedString = path.display().to_string().into();
                    app.invoke_drop_midi_file(path_str);
                    app.window().request_redraw();
                }
                EventResult::Propagate
            }
            winit::event::WindowEvent::HoveredFileCancelled => {
                if let Some(app) = app_weak.upgrade() {
                    app.set_drop_hovering(false);
                    app.window().request_redraw();
                }
                EventResult::Propagate
            }
            _ => EventResult::Propagate,
        });
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
