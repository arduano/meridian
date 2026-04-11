use std::{
    path::{Path, PathBuf},
    sync::MutexGuard,
};

use meridian_core::render::{
    NotePaletteConfig, NoteProjectorConfig, ProjectorBackgroundConfig, ProjectorImageConfig,
    SceneConfig, ThreeDSceneConfig, ZenithPaletteSpec,
};

use super::{schema::UiPreferences, LAST_AURA_PNG, LAST_BACKGROUND_PNG, LAST_PALETTE_PNG};

pub(super) fn active_palette_path_from_scene(scene: &SceneConfig) -> Option<PathBuf> {
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

pub(super) fn active_background_path_from_scene(scene: &SceneConfig) -> Option<String> {
    match scene.background() {
        ProjectorBackgroundConfig::PngFile { path, .. } => Some(path.clone()),
        ProjectorBackgroundConfig::None => None,
    }
}

pub(super) fn active_aura_path_from_scene(scene: &SceneConfig) -> Option<String> {
    let SceneConfig::ThreeD(ThreeDSceneConfig::PianoTrailClassic(config)) = scene else {
        return None;
    };
    match &config.aura_image {
        ProjectorImageConfig::PngFile { path } => Some(path.clone()),
        ProjectorImageConfig::Builtin { .. } => None,
    }
}

pub(super) fn restore_last_asset_paths(preferences: &UiPreferences) {
    *palette_slot() = preferences.last_palette_png.clone();
    *background_slot() = preferences.last_background_png.clone();
    *aura_slot() = preferences.last_aura_png.clone();
}

pub(super) fn last_palette_png() -> Option<PathBuf> {
    palette_slot().clone()
}

pub(super) fn last_background_png() -> Option<String> {
    background_slot().clone()
}

pub(super) fn last_aura_png() -> Option<String> {
    aura_slot().clone()
}

pub(super) fn file_name_or_path(path: &Path) -> String {
    path.file_name()
        .and_then(std::ffi::OsStr::to_str)
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| path.display().to_string())
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

fn palette_slot() -> MutexGuard<'static, Option<PathBuf>> {
    LAST_PALETTE_PNG.lock().expect("palette png mutex poisoned")
}

fn background_slot() -> MutexGuard<'static, Option<String>> {
    LAST_BACKGROUND_PNG
        .lock()
        .expect("background png mutex poisoned")
}

fn aura_slot() -> MutexGuard<'static, Option<String>> {
    LAST_AURA_PNG.lock().expect("aura png mutex poisoned")
}
