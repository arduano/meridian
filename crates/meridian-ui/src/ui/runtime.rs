use std::{
    cell::RefCell,
    ffi::OsStr,
    path::PathBuf,
    rc::Rc,
    sync::{Arc, Mutex},
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};

use meridian_core::{
    CoreHandle, MeridianError,
    midi::MidiProcessingConfig,
    protocol::{CoreEvent, ParsedMidiId, ProcessedMidiId},
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
    let preview_load_generation = Arc::new(AtomicU64::new(0));
    let render_load_generation = Arc::new(AtomicU64::new(0));
    let audio_load_generation = Arc::new(AtomicU64::new(0));
    let analysis_load_generation = Arc::new(AtomicU64::new(0));
    let pending_viewport_image = Rc::new(RefCell::new(None));
    let viewport_size = Rc::new(RefCell::new((1280_u32, 720_u32)));
    initialize_core(&bridge, &options, &app, &shared_state)?;
    install_load_progress_listener(&app, bridge.core());
    wire_callbacks(
        &app,
        &bridge,
        &shared_state,
        &preview_load_generation,
        &render_load_generation,
        &audio_load_generation,
        &analysis_load_generation,
    );
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
    if let Some(path) = &options.midi_path {
        let name: slint::SharedString = path.display().to_string().into();
        set_selected_midi(app, name);
    }
    bridge.initialize(options, shared_state)?;
    let initial = bridge.refresh_state(shared_state)?;
    apply_events_to_app(app, shared_state, &initial);
    Ok(())
}

fn wire_callbacks(
    app: &App,
    bridge: &UiCoreBridge,
    shared_state: &Arc<Mutex<UiViewModel>>,
    preview_load_generation: &Arc<AtomicU64>,
    render_load_generation: &Arc<AtomicU64>,
    audio_load_generation: &Arc<AtomicU64>,
    analysis_load_generation: &Arc<AtomicU64>,
) {
    wire_transport_callbacks(app, bridge, shared_state);
    wire_midi_callbacks(
        app,
        bridge,
        shared_state,
        preview_load_generation,
        render_load_generation,
        audio_load_generation,
        analysis_load_generation,
    );
}

fn app_is_loading(app: &App) -> bool {
    app.get_render_load_state() == MidiLoadState::Loading
        || app.get_audio_load_state() == MidiLoadState::Loading
        || app.get_analysis_load_state() == MidiLoadState::Loading
}

fn selected_or_empty_state(app: &App) -> MidiLoadState {
    if app.get_selected_midi_name().is_empty() {
        MidiLoadState::NoMidi
    } else {
        MidiLoadState::Selected
    }
}

fn set_selected_midi(app: &App, selected_midi_name: slint::SharedString) {
    app.set_selected_midi_name(selected_midi_name.clone());
    app.set_window_title(window_title_for_selected(selected_midi_name.as_str()));
}

fn window_title_for_selected(selected_midi_name: &str) -> slint::SharedString {
    let selected_path = PathBuf::from(selected_midi_name);
    let Some(file_name) = selected_path
        .file_name()
        .and_then(OsStr::to_str)
        .filter(|name| !name.is_empty())
    else {
        return "Meridian".into();
    };

    format!("Meridian - {file_name}").into()
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
fn wire_midi_callbacks(
    app: &App,
    bridge: &UiCoreBridge,
    shared_state: &Arc<Mutex<UiViewModel>>,
    preview_load_generation: &Arc<AtomicU64>,
    render_load_generation: &Arc<AtomicU64>,
    audio_load_generation: &Arc<AtomicU64>,
    analysis_load_generation: &Arc<AtomicU64>,
) {
    // ── Browse button (open native file dialog) ──
    {
        let app_weak = app.as_weak();
        let preview_load_generation = Arc::clone(preview_load_generation);
        let render_load_generation = Arc::clone(render_load_generation);
        let audio_load_generation = Arc::clone(audio_load_generation);
        let analysis_load_generation = Arc::clone(analysis_load_generation);
        app.on_select_midi_file(move || {
            let app_weak = app_weak.clone();
            let preview_load_generation = Arc::clone(&preview_load_generation);
            let render_load_generation = Arc::clone(&render_load_generation);
            let audio_load_generation = Arc::clone(&audio_load_generation);
            let analysis_load_generation = Arc::clone(&analysis_load_generation);
            // rfd's async dialog won't block the event loop on supported platforms
            std::thread::spawn(move || {
                let file = rfd::FileDialog::new()
                    .add_filter("MIDI files", &["mid", "midi", "MID", "MIDI"])
                    .add_filter("All files", &["*"])
                    .pick_file();
                if let Some(path) = file {
                    let name: slint::SharedString = path.display().to_string().into();
                    let _ = app_weak.upgrade_in_event_loop(move |app| {
                        preview_load_generation.fetch_add(1, Ordering::SeqCst);
                        render_load_generation.fetch_add(1, Ordering::SeqCst);
                        audio_load_generation.fetch_add(1, Ordering::SeqCst);
                        analysis_load_generation.fetch_add(1, Ordering::SeqCst);
                        app.set_viewport_image(slint::Image::default());
                        set_selected_midi(&app, name);
                        app.set_audio_load_state(MidiLoadState::Selected);
                        app.set_render_load_state(MidiLoadState::Selected);
                        app.set_analysis_load_state(MidiLoadState::Selected);
                        app.set_audio_load_error(Default::default());
                        app.set_render_load_error(Default::default());
                        app.set_analysis_load_error(Default::default());
                        app.set_audio_loading_progress(0.0);
                        app.set_audio_loading_status(Default::default());
                        app.set_render_loading_progress(0.0);
                        app.set_render_loading_status(Default::default());
                        app.set_analysis_loading_progress(0.0);
                        app.set_analysis_loading_status(Default::default());
                        reset_analysis_outputs(&app);
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
        let preview_load_generation = Arc::clone(preview_load_generation);
        let render_load_generation = Arc::clone(render_load_generation);
        let audio_load_generation = Arc::clone(audio_load_generation);
        let analysis_load_generation = Arc::clone(analysis_load_generation);
        app.on_drop_midi_file(move |path| {
            if let Some(app) = app_weak.upgrade() {
                app.set_viewport_image(slint::Image::default());
                set_selected_midi(&app, path.clone());
                preview_load_generation.fetch_add(1, Ordering::SeqCst);
                render_load_generation.fetch_add(1, Ordering::SeqCst);
                audio_load_generation.fetch_add(1, Ordering::SeqCst);
                analysis_load_generation.fetch_add(1, Ordering::SeqCst);
                app.set_audio_load_state(MidiLoadState::Selected);
                app.set_render_load_state(MidiLoadState::Selected);
                app.set_analysis_load_state(MidiLoadState::Selected);
                app.set_audio_load_error(Default::default());
                app.set_render_load_error(Default::default());
                app.set_analysis_load_error(Default::default());
                app.set_audio_loading_progress(0.0);
                app.set_audio_loading_status(Default::default());
                app.set_render_loading_progress(0.0);
                app.set_render_loading_status(Default::default());
                app.set_analysis_loading_progress(0.0);
                app.set_analysis_loading_status(Default::default());
                reset_analysis_outputs(&app);
                // Auto-load for the current profile context
                match app.get_active_profile() {
                    0 => load_preview_midi_async(
                        &app,
                        &bridge,
                        &shared_state,
                        &preview_load_generation,
                        &path,
                    ),
                    1 => load_render_midi_async(
                        &app,
                        &bridge,
                        &shared_state,
                        &render_load_generation,
                        &path,
                    ),
                    2 => load_audio_midi_async(
                        &app,
                        &bridge,
                        &shared_state,
                        &audio_load_generation,
                        &path,
                    ),
                    _ => load_analysis_midi_async(
                        &app,
                        &bridge,
                        &shared_state,
                        &analysis_load_generation,
                        &path,
                    ),
                }
            }
        });
    }

    // ── Load for preview ──
    {
        let app_weak = app.as_weak();
        let bridge = bridge.clone();
        let shared_state = Arc::clone(shared_state);
        let preview_load_generation = Arc::clone(preview_load_generation);
        app.on_load_for_preview(move || {
            if let Some(app) = app_weak.upgrade() {
                let midi_name = app.get_selected_midi_name();
                if !midi_name.is_empty() {
                    load_preview_midi_async(
                        &app,
                        &bridge,
                        &shared_state,
                        &preview_load_generation,
                        &midi_name,
                    );
                }
            }
        });
    }

    // ── Load for render ──
    {
        let app_weak = app.as_weak();
        let bridge = bridge.clone();
        let shared_state = Arc::clone(shared_state);
        let render_load_generation = Arc::clone(render_load_generation);
        app.on_load_for_render(move || {
            if let Some(app) = app_weak.upgrade() {
                let midi_name = app.get_selected_midi_name();
                if !midi_name.is_empty() {
                    load_render_midi_async(
                        &app,
                        &bridge,
                        &shared_state,
                        &render_load_generation,
                        &midi_name,
                    );
                }
            }
        });
    }

    // ── Load for audio ──
    {
        let app_weak = app.as_weak();
        let bridge = bridge.clone();
        let shared_state = Arc::clone(shared_state);
        let audio_load_generation = Arc::clone(audio_load_generation);
        app.on_load_for_audio(move || {
            if let Some(app) = app_weak.upgrade() {
                let midi_name = app.get_selected_midi_name();
                if !midi_name.is_empty() {
                    load_audio_midi_async(
                        &app,
                        &bridge,
                        &shared_state,
                        &audio_load_generation,
                        &midi_name,
                    );
                }
            }
        });
    }

    // ── Load for analysis / modify ──
    {
        let app_weak = app.as_weak();
        let bridge = bridge.clone();
        let shared_state = Arc::clone(shared_state);
        let analysis_load_generation = Arc::clone(analysis_load_generation);
        app.on_load_for_analysis(move || {
            if let Some(app) = app_weak.upgrade() {
                let midi_name = app.get_selected_midi_name();
                if !midi_name.is_empty() {
                    load_analysis_midi_async(
                        &app,
                        &bridge,
                        &shared_state,
                        &analysis_load_generation,
                        &midi_name,
                    );
                }
            }
        });
    }

    // ── Cancel preview load ──
    {
        let app_weak = app.as_weak();
        let preview_load_generation = Arc::clone(preview_load_generation);
        app.on_cancel_load_preview(move || {
            if let Some(app) = app_weak.upgrade() {
                preview_load_generation.fetch_add(1, Ordering::SeqCst);
                if app.get_render_load_state() == MidiLoadState::Loading {
                    app.set_render_load_state(MidiLoadState::Selected);
                }
                if app.get_audio_load_state() == MidiLoadState::Loading {
                    app.set_audio_load_state(MidiLoadState::Selected);
                }
                app.set_render_loading_progress(0.0);
                app.set_render_loading_status(Default::default());
                app.set_audio_loading_progress(0.0);
                app.set_audio_loading_status(Default::default());
            }
        });
    }

    // ── Cancel render load ──
    {
        let app_weak = app.as_weak();
        let render_load_generation = Arc::clone(render_load_generation);
        app.on_cancel_load_render(move || {
            if let Some(app) = app_weak.upgrade() {
                render_load_generation.fetch_add(1, Ordering::SeqCst);
                if app.get_render_load_state() == MidiLoadState::Loading {
                    app.set_render_load_state(MidiLoadState::Selected);
                }
                app.set_render_loading_progress(0.0);
                app.set_render_loading_status(Default::default());
            }
        });
    }

    // ── Cancel analysis load ──
    {
        let app_weak = app.as_weak();
        let analysis_load_generation = Arc::clone(analysis_load_generation);
        app.on_cancel_load_analysis(move || {
            if let Some(app) = app_weak.upgrade() {
                analysis_load_generation.fetch_add(1, Ordering::SeqCst);
                if app.get_analysis_load_state() == MidiLoadState::Loading {
                    app.set_analysis_load_state(MidiLoadState::Selected);
                }
                app.set_analysis_loading_progress(0.0);
                app.set_analysis_loading_status(Default::default());
            }
        });
    }

    // ── Cancel audio load ──
    {
        let app_weak = app.as_weak();
        let audio_load_generation = Arc::clone(audio_load_generation);
        app.on_cancel_load_audio(move || {
            if let Some(app) = app_weak.upgrade() {
                audio_load_generation.fetch_add(1, Ordering::SeqCst);
                if app.get_audio_load_state() == MidiLoadState::Loading {
                    app.set_audio_load_state(MidiLoadState::Selected);
                }
                app.set_audio_loading_progress(0.0);
                app.set_audio_loading_status(Default::default());
            }
        });
    }

    // ── Unload preview ──
    {
        let app_weak = app.as_weak();
        let bridge = bridge.clone();
        let shared_state = Arc::clone(shared_state);
        let preview_load_generation = Arc::clone(preview_load_generation);
        app.on_unload_preview(move || {
            if let Some(app) = app_weak.upgrade() {
                preview_load_generation.fetch_add(1, Ordering::SeqCst);
                if let Ok(events) = bridge.unload_render_context(&shared_state) {
                    apply_events_to_app(&app, &shared_state, &events);
                }
                app.set_viewport_image(slint::Image::default());
                app.set_render_load_state(selected_or_empty_state(&app));
                app.set_audio_load_state(selected_or_empty_state(&app));
                app.set_render_loading_progress(0.0);
                app.set_render_loading_status(Default::default());
                app.set_render_load_error(Default::default());
                app.set_audio_loading_progress(0.0);
                app.set_audio_loading_status(Default::default());
                app.set_audio_load_error(Default::default());
            }
        });
    }

    // ── Unload render ──
    {
        let app_weak = app.as_weak();
        let bridge = bridge.clone();
        let shared_state = Arc::clone(shared_state);
        let render_load_generation = Arc::clone(render_load_generation);
        app.on_unload_render(move || {
            if let Some(app) = app_weak.upgrade() {
                render_load_generation.fetch_add(1, Ordering::SeqCst);
                if let Ok(events) = bridge.unload_display_context(&shared_state) {
                    apply_events_to_app(&app, &shared_state, &events);
                }
                app.set_viewport_image(slint::Image::default());
                app.set_render_load_state(selected_or_empty_state(&app));
                app.set_render_loading_progress(0.0);
                app.set_render_loading_status(Default::default());
                app.set_render_load_error(Default::default());
            }
        });
    }

    // ── Unload audio ──
    {
        let app_weak = app.as_weak();
        let bridge = bridge.clone();
        let shared_state = Arc::clone(shared_state);
        let audio_load_generation = Arc::clone(audio_load_generation);
        app.on_unload_audio(move || {
            if let Some(app) = app_weak.upgrade() {
                audio_load_generation.fetch_add(1, Ordering::SeqCst);
                if let Ok(events) = bridge.unload_audio_context(&shared_state) {
                    apply_events_to_app(&app, &shared_state, &events);
                }
                app.set_audio_load_state(selected_or_empty_state(&app));
                app.set_audio_loading_progress(0.0);
                app.set_audio_loading_status(Default::default());
                app.set_audio_load_error(Default::default());
            }
        });
    }

    // ── Unload analysis ──
    {
        let app_weak = app.as_weak();
        let analysis_load_generation = Arc::clone(analysis_load_generation);
        app.on_unload_analysis(move || {
            if let Some(app) = app_weak.upgrade() {
                analysis_load_generation.fetch_add(1, Ordering::SeqCst);
                app.set_analysis_load_state(selected_or_empty_state(&app));
                app.set_analysis_loading_progress(0.0);
                app.set_analysis_loading_status(Default::default());
                app.set_analysis_load_error(Default::default());
                reset_analysis_outputs(&app);
            }
        });
    }

    // ── Retry load preview ──
    {
        let app_weak = app.as_weak();
        let bridge = bridge.clone();
        let shared_state = Arc::clone(shared_state);
        let preview_load_generation = Arc::clone(preview_load_generation);
        app.on_retry_load_preview(move || {
            if let Some(app) = app_weak.upgrade() {
                let midi_name = app.get_selected_midi_name();
                if !midi_name.is_empty() {
                    load_preview_midi_async(
                        &app,
                        &bridge,
                        &shared_state,
                        &preview_load_generation,
                        &midi_name,
                    );
                }
            }
        });
    }

    // ── Retry load render ──
    {
        let app_weak = app.as_weak();
        let bridge = bridge.clone();
        let shared_state = Arc::clone(shared_state);
        let render_load_generation = Arc::clone(render_load_generation);
        app.on_retry_load_render(move || {
            if let Some(app) = app_weak.upgrade() {
                let midi_name = app.get_selected_midi_name();
                if !midi_name.is_empty() {
                    load_render_midi_async(
                        &app,
                        &bridge,
                        &shared_state,
                        &render_load_generation,
                        &midi_name,
                    );
                }
            }
        });
    }

    // ── Retry load audio ──
    {
        let app_weak = app.as_weak();
        let bridge = bridge.clone();
        let shared_state = Arc::clone(shared_state);
        let audio_load_generation = Arc::clone(audio_load_generation);
        app.on_retry_load_audio(move || {
            if let Some(app) = app_weak.upgrade() {
                let midi_name = app.get_selected_midi_name();
                if !midi_name.is_empty() {
                    load_audio_midi_async(
                        &app,
                        &bridge,
                        &shared_state,
                        &audio_load_generation,
                        &midi_name,
                    );
                }
            }
        });
    }

    // ── Retry load analysis ──
    {
        let app_weak = app.as_weak();
        let bridge = bridge.clone();
        let shared_state = Arc::clone(shared_state);
        let analysis_load_generation = Arc::clone(analysis_load_generation);
        app.on_retry_load_analysis(move || {
            if let Some(app) = app_weak.upgrade() {
                let midi_name = app.get_selected_midi_name();
                if !midi_name.is_empty() {
                    load_analysis_midi_async(
                        &app,
                        &bridge,
                        &shared_state,
                        &analysis_load_generation,
                        &midi_name,
                    );
                }
            }
        });
    }
}

/// Start a preview MIDI load without blocking the Slint event loop.
fn load_preview_midi_async(
    app: &App,
    bridge: &UiCoreBridge,
    shared_state: &Arc<Mutex<UiViewModel>>,
    preview_load_generation: &Arc<AtomicU64>,
    midi_path: &str,
) {
    app.set_render_load_state(MidiLoadState::Loading);
    app.set_audio_load_state(MidiLoadState::Loading);
    app.set_render_loading_status("Loading preview playback…".into());
    app.set_audio_loading_status("Loading preview playback…".into());
    app.set_render_loading_progress(0.0);
    app.set_audio_loading_progress(0.0);
    app.set_render_load_error(Default::default());
    app.set_audio_load_error(Default::default());
    app.window().request_redraw();

    let path = PathBuf::from(midi_path.to_string());
    let requested_name: slint::SharedString = midi_path.into();
    let request_generation = preview_load_generation.fetch_add(1, Ordering::SeqCst) + 1;
    let app_weak = app.as_weak();
    let bridge = bridge.clone();
    let shared_state = Arc::clone(shared_state);
    let preview_load_generation = Arc::clone(preview_load_generation);
    std::thread::spawn(move || {
        let result = (|| -> Result<Vec<CoreEvent>, MeridianError> {
            let mut events = bridge.load_midi(path, &shared_state)?;
            events.extend(bridge.set_playing(true, &shared_state)?);
            Ok(events)
        })();
        if preview_load_generation.load(Ordering::SeqCst) != request_generation {
            return;
        }
        let _ = app_weak.upgrade_in_event_loop(move |app| {
            if preview_load_generation.load(Ordering::SeqCst) != request_generation {
                return;
            }
            if app.get_selected_midi_name() != requested_name {
                return;
            }
            match result {
                Ok(events) => {
                    apply_events_to_app(&app, &shared_state, &events);
                    app.set_render_load_state(MidiLoadState::Loaded);
                    app.set_audio_load_state(MidiLoadState::Loaded);
                    app.set_render_loading_progress(1.0);
                    app.set_audio_loading_progress(1.0);
                    app.set_render_loading_status("Preview ready".into());
                    app.set_audio_loading_status("Preview ready".into());
                }
                Err(e) => {
                    app.set_render_load_state(MidiLoadState::Error);
                    app.set_audio_load_state(MidiLoadState::Error);
                    app.set_render_load_error(e.to_string().into());
                    app.set_audio_load_error(e.to_string().into());
                    app.set_render_loading_status(Default::default());
                    app.set_audio_loading_status(Default::default());
                }
            }
            app.window().request_redraw();
        });
    });
}

/// Start a render-context MIDI load without blocking the Slint event loop.
fn load_render_midi_async(
    app: &App,
    bridge: &UiCoreBridge,
    shared_state: &Arc<Mutex<UiViewModel>>,
    render_load_generation: &Arc<AtomicU64>,
    midi_path: &str,
) {
    app.set_render_load_state(MidiLoadState::Loading);
    app.set_render_loading_status("Preparing display cache…".into());
    app.set_render_loading_progress(0.0);
    app.set_render_load_error(Default::default());
    app.window().request_redraw();

    let path = PathBuf::from(midi_path.to_string());
    let requested_name: slint::SharedString = midi_path.into();
    let request_generation = render_load_generation.fetch_add(1, Ordering::SeqCst) + 1;
    let app_weak = app.as_weak();
    let bridge = bridge.clone();
    let shared_state = Arc::clone(shared_state);
    let render_load_generation = Arc::clone(render_load_generation);
    std::thread::spawn(move || {
        let result = bridge.load_display_midi(path, &shared_state);
        if render_load_generation.load(Ordering::SeqCst) != request_generation {
            return;
        }
        let _ = app_weak.upgrade_in_event_loop(move |app| {
            if render_load_generation.load(Ordering::SeqCst) != request_generation {
                return;
            }
            if app.get_selected_midi_name() != requested_name {
                return;
            }
            match result {
                Ok(events) => {
                    apply_events_to_app(&app, &shared_state, &events);
                    app.set_render_load_state(MidiLoadState::Loaded);
                    app.set_render_loading_progress(1.0);
                    app.set_render_loading_status("Render visuals ready".into());
                }
                Err(e) => {
                    app.set_render_load_state(MidiLoadState::Error);
                    app.set_render_load_error(e.to_string().into());
                    app.set_render_loading_status(Default::default());
                }
            }
            app.window().request_redraw();
        });
    });
}

fn load_audio_midi_async(
    app: &App,
    bridge: &UiCoreBridge,
    shared_state: &Arc<Mutex<UiViewModel>>,
    audio_load_generation: &Arc<AtomicU64>,
    midi_path: &str,
) {
    app.set_audio_load_state(MidiLoadState::Loading);
    app.set_audio_loading_status("Preparing audio cache…".into());
    app.set_audio_loading_progress(0.0);
    app.set_audio_load_error(Default::default());
    app.window().request_redraw();

    let path = PathBuf::from(midi_path.to_string());
    let requested_name: slint::SharedString = midi_path.into();
    let request_generation = audio_load_generation.fetch_add(1, Ordering::SeqCst) + 1;
    let app_weak = app.as_weak();
    let bridge = bridge.clone();
    let shared_state = Arc::clone(shared_state);
    let audio_load_generation = Arc::clone(audio_load_generation);
    std::thread::spawn(move || {
        let result = bridge.load_audio_midi(path, &shared_state);
        if audio_load_generation.load(Ordering::SeqCst) != request_generation {
            return;
        }
        let _ = app_weak.upgrade_in_event_loop(move |app| {
            if audio_load_generation.load(Ordering::SeqCst) != request_generation {
                return;
            }
            if app.get_selected_midi_name() != requested_name {
                return;
            }
            match result {
                Ok(events) => {
                    apply_events_to_app(&app, &shared_state, &events);
                    app.set_audio_load_state(MidiLoadState::Loaded);
                    app.set_audio_loading_progress(1.0);
                    app.set_audio_loading_status("Audio ready".into());
                }
                Err(e) => {
                    app.set_audio_load_state(MidiLoadState::Error);
                    app.set_audio_load_error(e.to_string().into());
                    app.set_audio_loading_status(Default::default());
                }
            }
            app.window().request_redraw();
        });
    });
}

fn load_analysis_midi_async(
    app: &App,
    bridge: &UiCoreBridge,
    shared_state: &Arc<Mutex<UiViewModel>>,
    analysis_load_generation: &Arc<AtomicU64>,
    midi_path: &str,
) {
    app.set_analysis_load_state(MidiLoadState::Loading);
    app.set_analysis_loading_status("Reading MIDI for analysis…".into());
    app.set_analysis_loading_progress(0.0);
    app.set_analysis_load_error(Default::default());
    reset_analysis_outputs(app);
    app.window().request_redraw();

    let path = PathBuf::from(midi_path.to_string());
    let requested_name: slint::SharedString = midi_path.into();
    let request_generation = analysis_load_generation.fetch_add(1, Ordering::SeqCst) + 1;
    let app_weak = app.as_weak();
    let bridge = bridge.clone();
    let shared_state = Arc::clone(shared_state);
    let analysis_load_generation = Arc::clone(analysis_load_generation);
    std::thread::spawn(move || {
        let progress = |value: f32, status: &'static str| {
            update_analysis_progress(
                &app_weak,
                &requested_name,
                &analysis_load_generation,
                request_generation,
                value,
                status,
            );
        };
        progress(0.15, "Parsing MIDI for analysis…");
        let result = load_analysis_resource_set(&bridge, &shared_state, &path, progress);
        if analysis_load_generation.load(Ordering::SeqCst) != request_generation {
            return;
        }
        let _ = app_weak.upgrade_in_event_loop(move |app| {
            if analysis_load_generation.load(Ordering::SeqCst) != request_generation {
                return;
            }
            if app.get_selected_midi_name() != requested_name {
                return;
            }
            match result {
                Ok(events) => {
                    apply_events_to_app(&app, &shared_state, &events);
                    app.set_analysis_load_state(MidiLoadState::Loaded);
                    app.set_analysis_loading_progress(1.0);
                    app.set_analysis_loading_status("Analysis ready".into());
                }
                Err(e) => {
                    app.set_analysis_load_state(MidiLoadState::Error);
                    app.set_analysis_load_error(e.to_string().into());
                    app.set_analysis_loading_status(Default::default());
                }
            }
            app.window().request_redraw();
        });
    });
}

fn load_analysis_resource_set(
    bridge: &UiCoreBridge,
    shared_state: &Arc<Mutex<UiViewModel>>,
    path: &std::path::Path,
    mut progress: impl FnMut(f32, &'static str),
) -> Result<Vec<CoreEvent>, MeridianError> {
    let mut all_events = Vec::new();

    let parsed_events = bridge.load_parsed_midi(path.to_path_buf(), shared_state)?;
    let parsed_midi_id = parsed_midi_id_from_events(&parsed_events)?;
    all_events.extend(parsed_events);

    progress(0.55, "Building analysis model…");
    let processed_events =
        bridge.build_processed_midi(parsed_midi_id, MidiProcessingConfig::default(), shared_state)?;
    let (processed_midi_id, midi_length) = processed_result_from_events(&processed_events)?;
    all_events.extend(processed_events);

    let bucket_count = ((midi_length / 0.5).ceil() as usize).clamp(1, 8192);
    progress(0.82, "Computing bucketed note statistics…");
    let analysis_events = bridge.analyze_processed_midi(processed_midi_id, Some(bucket_count), shared_state)?;
    all_events.extend(analysis_events);

    Ok(all_events)
}

fn parsed_midi_id_from_events(events: &[CoreEvent]) -> Result<ParsedMidiId, MeridianError> {
    events
        .iter()
        .find_map(|event| match event {
            CoreEvent::ParsedMidiLoaded { parsed_midi_id, .. } => Some(*parsed_midi_id),
            _ => None,
        })
        .ok_or_else(|| MeridianError::InvalidMidi("missing parsed MIDI id".into()))
}

fn processed_result_from_events(
    events: &[CoreEvent],
) -> Result<(ProcessedMidiId, f64), MeridianError> {
    events
        .iter()
        .find_map(|event| match event {
            CoreEvent::ProcessedMidiBuilt {
                processed_midi_id,
                midi_length,
                ..
            } => Some((*processed_midi_id, *midi_length)),
            _ => None,
        })
        .ok_or_else(|| MeridianError::InvalidMidi("missing processed MIDI id".into()))
}

fn reset_analysis_outputs(app: &App) {
    app.set_analysis_note_count_text("—".into());
    app.set_analysis_track_count_text("—".into());
    app.set_analysis_midi_length_text("—".into());
    app.set_analysis_tempo_text("—".into());
    app.set_analysis_time_signature_text("—".into());
    app.set_analysis_key_range_text("—".into());
    app.set_analysis_avg_velocity_text("—".into());
    app.set_analysis_note_density_text("—".into());
}

fn update_analysis_progress(
    app_weak: &slint::Weak<App>,
    requested_name: &slint::SharedString,
    analysis_load_generation: &Arc<AtomicU64>,
    request_generation: u64,
    progress: f32,
    status: &str,
) {
    let app_weak = app_weak.clone();
    let requested_name = requested_name.clone();
    let analysis_load_generation = Arc::clone(analysis_load_generation);
    let status_text: slint::SharedString = status.into();
    let _ = app_weak.upgrade_in_event_loop(move |app| {
        if analysis_load_generation.load(Ordering::SeqCst) != request_generation {
            return;
        }
        if app.get_selected_midi_name() != requested_name {
            return;
        }
        if app.get_analysis_load_state() != MidiLoadState::Loading {
            return;
        }
        app.set_analysis_loading_progress(progress);
        app.set_analysis_loading_status(status_text);
        app.window().request_redraw();
    });
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

fn install_load_progress_listener(app: &App, core: &CoreHandle) {
    let receiver = core.subscribe_events();
    let app_weak = app.as_weak();
    std::thread::spawn(move || {
        for event in receiver {
            let CoreEvent::MidiLoadProgress {
                path,
                progress,
                status,
            } = event
            else {
                continue;
            };

            let path_text: slint::SharedString = path.display().to_string().into();
            let status_text: slint::SharedString = status.into();
            let _ = app_weak.upgrade_in_event_loop(move |app| {
                if app.get_selected_midi_name() != path_text {
                    return;
                }
                if app.get_render_load_state() == MidiLoadState::Loading {
                    app.set_render_loading_progress(progress);
                    app.set_render_loading_status(status_text.clone());
                }
                if app.get_audio_load_state() == MidiLoadState::Loading {
                    app.set_audio_loading_progress(progress);
                    app.set_audio_loading_status(status_text);
                }
                app.window().request_redraw();
            });
        }
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
                if !app_is_loading(&app) {
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
                }
                if disable_wgpu || app.get_play_label() == "Pause" {
                    app.window().request_redraw();
                }
            }
        },
    );
    animation_timer
}
