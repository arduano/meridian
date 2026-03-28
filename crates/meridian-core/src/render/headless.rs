use std::path::Path;

use crate::{
    error::MeridianError,
    protocol::ImageOutputFormat,
    render::{SceneConfig, SceneLayout, shared::ProjectedScene},
};

pub enum HeadlessRenderSession {
    TwoD(crate::render::pfa::wgpu::HeadlessRenderSession),
    ThreeD(crate::render::miditrail::wgpu::HeadlessRenderSession),
}

impl HeadlessRenderSession {
    pub fn new(layout: &SceneLayout, width: u32, height: u32) -> Result<Self, MeridianError> {
        match &layout.scene {
            SceneConfig::TwoD(_) => Ok(Self::TwoD(
                crate::render::pfa::wgpu::HeadlessRenderSession::new(width, height)?,
            )),
            SceneConfig::ThreeD(_) => Ok(Self::ThreeD(
                crate::render::miditrail::wgpu::HeadlessRenderSession::new(width, height)?,
            )),
        }
    }

    pub fn render(&mut self, layout: &SceneLayout, scene: &ProjectedScene) {
        match self {
            Self::TwoD(session) => session.render(scene),
            Self::ThreeD(session) => {
                if let Some(miditrail) = scene.miditrail() {
                    session.render(layout, miditrail);
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
        SceneConfig::ThreeD(_) => {
            let miditrail = scene.miditrail().ok_or_else(|| {
                MeridianError::InvalidMidi("missing miditrail scene payload".into())
            })?;
            crate::render::miditrail::wgpu::save_scene_headless(
                width, height, layout, miditrail, format, output,
            )
        }
    }
}
