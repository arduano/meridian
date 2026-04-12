use std::{
    path::PathBuf,
};

use meridian_core::render::{
    NotePaletteConfig, NoteProjectorConfig, ProjectorBackgroundConfig, ProjectorImageConfig,
    SceneConfig, ThreeDSceneConfig, ZenithPaletteSpec,
};

use super::schema::UiPreferences;
use super::super::UiViewModel;

pub(crate) fn active_palette_path_from_scene(scene: &SceneConfig) -> Option<PathBuf> {
    match scene {
        SceneConfig::TwoD(config) => active_palette_path_from_notes(&config.notes),
        SceneConfig::ThreeD(ThreeDSceneConfig::PianoTrailClassic(config)) => {
            match &config.palette {
                NotePaletteConfig::ZenithPalette {
                    palette: ZenithPaletteSpec::PngFile { path },
                    ..
                } => Some(path.clone()),
                _ => None,
            }
        }
    }
}

pub(crate) fn active_background_path_from_scene(scene: &SceneConfig) -> Option<String> {
    match scene.background() {
        ProjectorBackgroundConfig::PngFile { path, .. } => Some(path.clone()),
        ProjectorBackgroundConfig::None => None,
    }
}

pub(crate) fn active_aura_path_from_scene(scene: &SceneConfig) -> Option<String> {
    let SceneConfig::ThreeD(ThreeDSceneConfig::PianoTrailClassic(config)) = scene else {
        return None;
    };
    match &config.aura_image {
        ProjectorImageConfig::PngFile { path } => Some(path.clone()),
        ProjectorImageConfig::Builtin { .. } => None,
    }
}

pub(crate) fn restore_last_asset_paths(
    state: &mut UiViewModel,
    preferences: &UiPreferences,
) {
    state.remembered_assets.palette_png = preferences.last_palette_png.clone();
    state.remembered_assets.background_png = preferences.last_background_png.clone();
    state.remembered_assets.aura_png = preferences.last_aura_png.clone();
}

pub(crate) fn remember_palette_png(state: &mut UiViewModel, path: PathBuf) {
    state.remembered_assets.palette_png = Some(path);
}

pub(crate) fn remember_background_png(state: &mut UiViewModel, path: String) {
    state.remembered_assets.background_png = Some(path);
}

pub(crate) fn remember_aura_png(state: &mut UiViewModel, path: String) {
    state.remembered_assets.aura_png = Some(path);
}

fn active_palette_path_from_notes(notes: &NoteProjectorConfig) -> Option<PathBuf> {
    let palette = match notes {
        NoteProjectorConfig::Flat(notes) => &notes.palette,
        NoteProjectorConfig::Pfa(notes) => &notes.palette,
    };
    match palette {
        NotePaletteConfig::ZenithPalette {
            palette: ZenithPaletteSpec::PngFile { path },
            ..
        } => Some(path.clone()),
        _ => None,
    }
}
