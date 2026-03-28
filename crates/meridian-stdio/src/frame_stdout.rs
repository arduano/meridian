use std::io::{self, Write};

use meridian_core::{
    MeridianError,
    protocol::{CoreCommand, ImageOutputFormat},
    render::{
        RendererKind, SceneLayout,
        pfa::wgpu::{encode_rgba_to_png, encode_rgba_to_ppm, render_scene_headless_to_rgba},
    },
    spawn_core,
};

pub fn run(
    midi: &std::path::Path,
    format: ImageOutputFormat,
    time: f64,
    view_range: f64,
    first_key: u8,
    last_key: u8,
    width: u32,
    height: u32,
    renderer: RendererKind,
) -> Result<(), MeridianError> {
    let core = spawn_core();
    let mut layout = SceneLayout::default();
    layout.set_renderer_kind(renderer);
    core.request(CoreCommand::SetSceneConfig {
        scene: layout.scene.clone(),
    })?;
    core.request(CoreCommand::SetViewRange {
        seconds: view_range,
    })?;
    core.request(CoreCommand::SetKeyRange {
        first_key,
        last_key,
    })?;
    core.request(CoreCommand::SetViewport { width, height })?;
    core.request(CoreCommand::LoadMidi {
        path: midi.to_path_buf(),
    })?;
    core.request(CoreCommand::SetTime { time })?;

    let frame = core.render_frame(Some(width), Some(height))?;
    let rgba = render_scene_headless_to_rgba(width, height, &frame.scene)?;
    let bytes = match format {
        ImageOutputFormat::Rgba => rgba,
        ImageOutputFormat::Ppm => encode_rgba_to_ppm(width, height, &rgba),
        ImageOutputFormat::Png => encode_rgba_to_png(width, height, &rgba)?,
    };

    let mut stdout = io::stdout().lock();
    stdout.write_all(&bytes)?;
    stdout.flush()?;
    let _ = core.request(CoreCommand::Shutdown);
    Ok(())
}
