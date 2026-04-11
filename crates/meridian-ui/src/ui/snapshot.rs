use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::Duration,
};

use meridian_core::{
    MeridianError,
    render::{SceneLayout, headless::render_scene_headless_to_rgba, pfa::wgpu::encode_rgba_to_png},
    spawn_core,
};
use slint::{ComponentHandle, Image, PhysicalSize, Rgba8Pixel, SharedPixelBuffer};

use super::{
    core_bridge::UiCoreBridge,
    runtime::initialize_core,
    state::{UiOptions, UiStartupOptions},
    view::App,
    view_model::UiViewModel,
};

pub fn write_debug_snapshot(
    options: UiOptions,
    output: &Path,
    window_width: u32,
    window_height: u32,
) -> Result<(), MeridianError> {
    let backend_selector = slint::BackendSelector::new();
    if options.disable_wgpu {
        backend_selector
            .backend_name("winit-software".into())
            .select()
            .map_err(|e| MeridianError::Platform(e.to_string()))?;
    } else {
        backend_selector
            .require_wgpu_28(slint::wgpu_28::WGPUConfiguration::default())
            .select()
            .map_err(|e| MeridianError::Platform(e.to_string()))?;
    }

    let app = App::new().map_err(|e| MeridianError::Platform(e.to_string()))?;
    app.window()
        .set_size(PhysicalSize::new(window_width.max(1), window_height.max(1)));

    let bridge = UiCoreBridge::new(spawn_core());
    let shared_state = Arc::new(Mutex::new(UiViewModel::default()));
    let mut startup = UiStartupOptions {
        disable_wgpu: options.disable_wgpu,
        ..UiStartupOptions::default()
    };
    if let Some(renderer) = options.renderer {
        let mut layout = SceneLayout {
            scene: startup.scene.clone(),
            ..SceneLayout::default()
        };
        layout.set_renderer_kind(renderer);
        startup.scene = layout.scene;
    }
    if let Some(midi_path) = &options.midi_path {
        startup.midi_path = Some(midi_path.clone());
    }
    if let Some(start_time) = options.start_time {
        startup.start_time = start_time.max(0.0);
    }
    if let Some(view_range) = options.view_range {
        startup.view_range = view_range.max(meridian_core::display::MIN_VIEW_RANGE_SECONDS);
    }
    if let Some(first_key) = options.first_key {
        startup.first_key = first_key;
    }
    if let Some(last_key) = options.last_key {
        startup.last_key = last_key;
    }
    startup.first_key = startup.first_key.min(startup.last_key);
    startup.last_key = startup.last_key.max(startup.first_key);
    initialize_core(&bridge, &startup, &app, &shared_state)?;
    inject_headless_viewport(&app, bridge.core())?;

    let output = output.to_path_buf();
    schedule_snapshot(app.as_weak(), output);
    app.show()
        .map_err(|e| MeridianError::Platform(e.to_string()))?;
    app.window().request_redraw();
    slint::run_event_loop_until_quit().map_err(|e| MeridianError::Platform(e.to_string()))?;
    Err(MeridianError::Platform(
        "debug snapshot exited the event loop without terminating".into(),
    ))
}

fn inject_headless_viewport(
    app: &App,
    core: &meridian_core::CoreHandle,
) -> Result<(), MeridianError> {
    let frame = core.render_frame(Some(1280), Some(720))?;
    let rgba = render_scene_headless_to_rgba(
        frame.layout.viewport_width,
        frame.layout.viewport_height,
        &frame.layout,
        &frame.scene,
    )?;
    let buffer = SharedPixelBuffer::<Rgba8Pixel>::clone_from_slice(
        &rgba,
        frame.layout.viewport_width,
        frame.layout.viewport_height,
    );
    app.set_viewport_image(Image::from_rgba8(buffer));
    Ok(())
}

fn schedule_snapshot(app_weak: slint::Weak<App>, output: PathBuf) {
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_secs(1));
        let _ = app_weak.upgrade_in_event_loop(move |app| {
            let exit_code = match capture_snapshot(&app, &output) {
                Ok(()) => 0,
                Err(error) => {
                    eprintln!("debug snapshot failed: {error}");
                    1
                }
            };
            std::process::exit(exit_code);
        });
    });
}

fn capture_snapshot(app: &App, output: &Path) -> Result<(), MeridianError> {
    app.window().request_redraw();
    let snapshot = app
        .window()
        .take_snapshot()
        .map_err(|e| MeridianError::Platform(e.to_string()))?;
    let flattened = flatten_snapshot(snapshot.as_bytes());
    let png = encode_rgba_to_png(snapshot.width(), snapshot.height(), &flattened)?;
    fs::write(output, png)?;
    Ok(())
}

fn flatten_snapshot(rgba: &[u8]) -> Vec<u8> {
    const BG_R: u32 = 8;
    const BG_G: u32 = 14;
    const BG_B: u32 = 24;

    let mut out = Vec::with_capacity(rgba.len());
    for pixel in rgba.chunks_exact(4) {
        let alpha = pixel[3] as u32;
        let inv_alpha = 255 - alpha;
        let r = (pixel[0] as u32 * alpha + BG_R * inv_alpha) / 255;
        let g = (pixel[1] as u32 * alpha + BG_G * inv_alpha) / 255;
        let b = (pixel[2] as u32 * alpha + BG_B * inv_alpha) / 255;
        out.extend_from_slice(&[r as u8, g as u8, b as u8, 255]);
    }
    out
}
