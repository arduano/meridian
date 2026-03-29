use std::path::Path;

use crate::{
    error::MeridianError,
    protocol::ImageOutputFormat,
    render::{SceneConfig, SceneLayout, ThreeDSceneConfig, shared::ProjectedScene},
};

pub enum HeadlessRenderSession {
    TwoD(crate::render::pfa::wgpu::HeadlessRenderSession),
    ThreeD(crate::render::piano_trail_classic::wgpu::HeadlessRenderSession),
}

impl HeadlessRenderSession {
    pub fn new(layout: &SceneLayout, width: u32, height: u32) -> Result<Self, MeridianError> {
        match &layout.scene {
            SceneConfig::TwoD(_) => Ok(Self::TwoD(
                crate::render::pfa::wgpu::HeadlessRenderSession::new(width, height)?,
            )),
            SceneConfig::ThreeD(ThreeDSceneConfig::PianoTrailClassic(_)) => Ok(Self::ThreeD(
                crate::render::piano_trail_classic::wgpu::HeadlessRenderSession::new(
                    width, height,
                )?,
            )),
        }
    }

    pub fn render(&mut self, layout: &SceneLayout, scene: &ProjectedScene) {
        match self {
            Self::TwoD(session) => session.render(scene),
            Self::ThreeD(session) => {
                if let Some(piano_trail_classic) = scene.piano_trail_classic() {
                    session.render(layout, piano_trail_classic);
                }
            }
        }
    }

    pub fn readback_rgba(&self) -> Result<Vec<u8>, MeridianError> {
        match self {
            Self::TwoD(session) => session.readback_rgba(),
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

pub fn save_scene_headless(
    width: u32,
    height: u32,
    layout: &SceneLayout,
    scene: &ProjectedScene,
    format: ImageOutputFormat,
    output: &Path,
) -> Result<u64, MeridianError> {
    match &layout.scene {
        SceneConfig::TwoD(_) => {
            crate::render::pfa::wgpu::save_scene_headless(width, height, scene, format, output)
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
