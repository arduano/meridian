use std::path::Path;

use crate::{
    error::MeridianError,
    protocol::ImageOutputFormat,
    render::{
        SceneConfig, SceneLayout, ThreeDSceneConfig, export::ExportFrame, shared::ProjectedScene,
    },
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HeadlessClearMode {
    OpaquePreview,
    Transparent,
}

pub enum HeadlessRenderSession {
    Primitive(crate::render::pfa::wgpu::HeadlessRenderSession),
    ThreeD(crate::render::piano_trail_classic::wgpu::HeadlessRenderSession),
}

impl HeadlessRenderSession {
    pub fn new(layout: &SceneLayout, width: u32, height: u32) -> Result<Self, MeridianError> {
        Self::new_with_clear_mode(layout, width, height, HeadlessClearMode::OpaquePreview)
    }

    pub fn new_with_clear_mode(
        layout: &SceneLayout,
        width: u32,
        height: u32,
        clear_mode: HeadlessClearMode,
    ) -> Result<Self, MeridianError> {
        match &layout.scene {
            SceneConfig::TwoD(_) | SceneConfig::Text(_) => Ok(Self::Primitive(
                crate::render::pfa::wgpu::HeadlessRenderSession::new_with_clear_mode(
                    width, height, clear_mode,
                )?,
            )),
            SceneConfig::ThreeD(ThreeDSceneConfig::PianoTrailClassic(_)) => Ok(Self::ThreeD(
                crate::render::piano_trail_classic::wgpu::HeadlessRenderSession::new_with_clear_mode(
                    width, height, clear_mode,
                )?,
            )),
        }
    }

    pub fn render(&mut self, layout: &SceneLayout, scene: &ProjectedScene) {
        match self {
            Self::Primitive(session) => session.render(layout, scene),
            Self::ThreeD(session) => {
                if let Some(piano_trail_classic) = scene.piano_trail_classic() {
                    session.render(layout, piano_trail_classic);
                }
            }
        }
    }

    pub fn readback_rgba(&self) -> Result<Vec<u8>, MeridianError> {
        match self {
            Self::Primitive(session) => session.readback_rgba(),
            Self::ThreeD(session) => session.readback_rgba(),
        }
    }
}

pub fn render_scene_headless_to_rgba(
    width: u32,
    height: u32,
    layout: &SceneLayout,
    scene: &ProjectedScene,
) -> Result<Vec<u8>, MeridianError> {
    let mut session = HeadlessRenderSession::new(layout, width, height)?;
    session.render(layout, scene);
    session.readback_rgba()
}

pub fn render_scene_headless_to_premultiplied_rgba(
    width: u32,
    height: u32,
    layout: &SceneLayout,
    scene: &ProjectedScene,
) -> Result<Vec<u8>, MeridianError> {
    let mut session = HeadlessRenderSession::new_with_clear_mode(
        layout,
        width,
        height,
        HeadlessClearMode::Transparent,
    )?;
    session.render(layout, scene);
    session.readback_rgba()
}

pub fn save_scene_headless(
    width: u32,
    height: u32,
    layout: &SceneLayout,
    scene: &ProjectedScene,
    format: ImageOutputFormat,
    output: &Path,
    export: &crate::protocol::ImageExportConfig,
) -> Result<(u64, crate::protocol::ImageExportArtifacts), MeridianError> {
    let rgba = render_scene_headless_to_premultiplied_rgba(width, height, layout, scene)?;
    let frame = ExportFrame::new(width, height, rgba);
    let bytes_written = frame.write_color_output(output, format, export.color_mode)?;
    let exports = frame.write_requested_sidecars(
        output,
        export.export_premultiplied_rgb,
        export.export_straight_rgb,
        export.export_alpha_mask,
    )?;
    Ok((bytes_written, exports))
}

pub fn save_scene_headless_legacy(
    width: u32,
    height: u32,
    layout: &SceneLayout,
    scene: &ProjectedScene,
    format: ImageOutputFormat,
    output: &Path,
) -> Result<u64, MeridianError> {
    match &layout.scene {
        SceneConfig::TwoD(_) | SceneConfig::Text(_) => {
            crate::render::pfa::wgpu::save_scene_headless(
                width, height, layout, scene, format, output,
            )
        }
        SceneConfig::ThreeD(ThreeDSceneConfig::PianoTrailClassic(_)) => {
            let piano_trail_classic = scene.piano_trail_classic().ok_or_else(|| {
                MeridianError::InvalidMidi("missing piano_trail_classic scene payload".into())
            })?;
            crate::render::piano_trail_classic::wgpu::save_scene_headless(
                width,
                height,
                layout,
                piano_trail_classic,
                format,
                output,
            )
        }
    }
}
