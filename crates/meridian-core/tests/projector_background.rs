mod support;

use std::{fs, io::Cursor};

use meridian_core::{
    protocol::CoreCommand,
    render::{
        ProjectorBackgroundConfig, ProjectorBackgroundScalingMode, RendererKind, SceneLayout,
        headless::render_scene_headless_to_rgba,
    },
    spawn_core,
};
use png::{BitDepth, ColorType, Encoder};

fn write_solid_png(name: &str, rgba: [u8; 4]) -> String {
    let dir = support::temp_dir("meridian-core-background");
    let path = dir.join(name);
    let mut bytes = Vec::new();
    {
        let writer = Cursor::new(&mut bytes);
        let mut encoder = Encoder::new(writer, 1, 1);
        encoder.set_color(ColorType::Rgba);
        encoder.set_depth(BitDepth::Eight);
        let mut png = encoder.write_header().expect("write png header");
        png.write_image_data(&rgba).expect("write png image");
    }
    fs::write(&path, bytes).expect("write background png");
    path.display().to_string()
}

#[test]
fn renderer_switch_preserves_shared_background() {
    let path = write_solid_png("shared-background.png", [255, 0, 0, 255]);
    let mut layout = SceneLayout::default();
    *layout.scene.background_mut() = ProjectorBackgroundConfig::PngFile {
        path: path.clone(),
        scaling: ProjectorBackgroundScalingMode::Cover,
    };

    layout.set_renderer_kind(RendererKind::Flat);
    assert_eq!(
        layout.scene.background(),
        &ProjectorBackgroundConfig::PngFile {
            path: path.clone(),
            scaling: ProjectorBackgroundScalingMode::Cover,
        }
    );

    layout.set_renderer_kind(RendererKind::PianoTrailClassic);
    assert_eq!(
        layout.scene.background(),
        &ProjectorBackgroundConfig::PngFile {
            path,
            scaling: ProjectorBackgroundScalingMode::Cover,
        }
    );
}

#[test]
fn headless_background_renders_for_all_projectors() {
    let midi = support::write_test_midi();
    let path = write_solid_png("render-background.png", [255, 0, 0, 255]);

    for renderer in [
        RendererKind::Flat,
        RendererKind::Pfa,
        RendererKind::PianoTrailClassic,
    ] {
        let core = spawn_core();
        core.request(CoreCommand::LoadMidi { path: midi.clone() })
            .expect("load midi");
        core.request(CoreCommand::SetTime { time: 0.25 })
            .expect("set time");
        core.request(CoreCommand::SetViewport {
            width: 96,
            height: 96,
        })
        .expect("set viewport");

        let mut layout = SceneLayout::default();
        *layout.scene.background_mut() = ProjectorBackgroundConfig::PngFile {
            path: path.clone(),
            scaling: ProjectorBackgroundScalingMode::Stretch,
        };
        layout.set_renderer_kind(renderer);
        core.request(CoreCommand::SetSceneConfig {
            scene: layout.scene.clone(),
        })
        .expect("set scene config");

        let frame = core
            .render_frame(Some(96), Some(96))
            .expect("render frame with background");
        let rgba = render_scene_headless_to_rgba(96, 96, &frame.layout, &frame.scene)
            .expect("render headless rgba");
        let pixel = &rgba[..4];
        assert!(
            pixel[0] > 200 && pixel[1] < 40 && pixel[2] < 40,
            "expected red background for {renderer:?}, got {pixel:?}"
        );
    }
}
