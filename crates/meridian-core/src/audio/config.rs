use std::{
    fs, io,
    path::{Path, PathBuf},
    sync::OnceLock,
};

use serde::{Deserialize, Serialize};
use xsynth_core::{
    AudioStreamParams, ChannelCount,
    channel_group::{ParallelismOptions, ThreadCount},
    soundfont::{EnvelopeCurveType, Interpolator, SoundfontInitOptions},
};
use xsynth_realtime::XSynthRealtimeConfig;

pub const EMBEDDED_DEFAULT_SOUNDFONT_ASSET: &str =
    "assets/soundfonts/freepats-upright-kw-small/UprightPianoKW-small-20190703.sf2";
pub const EMBEDDED_DEFAULT_SOUNDFONT_NAME: &str = "UprightPianoKW-small-20190703.sf2";

static EMBEDDED_DEFAULT_SOUNDFONT_BYTES: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../",
    "assets/soundfonts/freepats-upright-kw-small/UprightPianoKW-small-20190703.sf2"
));
static EMBEDDED_DEFAULT_SOUNDFONT_PATH: OnceLock<PathBuf> = OnceLock::new();

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AudioBackend {
    None,
    Xsynth,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct MeridianSoundfont {
    pub path: Option<PathBuf>,
    pub enabled: bool,
    pub options: SoundfontInitOptions,
}

impl Default for MeridianSoundfont {
    fn default() -> Self {
        Self {
            path: None,
            enabled: true,
            options: Default::default(),
        }
    }
}

impl MeridianSoundfont {
    pub fn uses_default(&self) -> bool {
        self.path.is_none()
    }

    pub fn path_ref(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    pub fn resolved_path(&self) -> io::Result<PathBuf> {
        match &self.path {
            Some(path) => Ok(path.clone()),
            None => resolve_default_soundfont_path(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct XSynthSettings {
    pub config: XSynthRealtimeConfig,
    pub limit_layers: bool,
    pub layers: usize,
    pub render: XSynthRenderSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct XSynthRenderSettings {
    pub audio_params: AudioStreamParams,
    pub parallelism: ParallelismOptions,
    pub use_limiter: bool,
}

impl Default for XSynthSettings {
    fn default() -> Self {
        Self {
            config: XSynthRealtimeConfig {
                multithreading: ThreadCount::Auto,
                ignore_range: 1..=7,
                ..Default::default()
            },
            limit_layers: true,
            layers: 4,
            render: Default::default(),
        }
    }
}

impl Default for XSynthRenderSettings {
    fn default() -> Self {
        Self {
            audio_params: AudioStreamParams::new(48_000, ChannelCount::Stereo),
            parallelism: ParallelismOptions {
                channel: ThreadCount::Auto,
                key: ThreadCount::Auto,
            },
            use_limiter: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct AudioConfig {
    pub backend: AudioBackend,
    pub soundfonts: Vec<MeridianSoundfont>,
    pub xsynth: XSynthSettings,
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            backend: AudioBackend::None,
            soundfonts: vec![MeridianSoundfont::default()],
            xsynth: Default::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub(crate) struct SoundfontCacheKey {
    pub path: PathBuf,
    pub bank: Option<u8>,
    pub preset: Option<u8>,
    pub use_effects: bool,
    pub interpolator: u8,
    pub attack_curve: u8,
    pub decay_curve: u8,
    pub release_curve: u8,
    pub sample_rate: u32,
    pub channels: u16,
}

impl SoundfontCacheKey {
    pub(crate) fn new(
        soundfont: &MeridianSoundfont,
        params: xsynth_core::AudioStreamParams,
    ) -> io::Result<Self> {
        Ok(Self {
            path: soundfont.resolved_path()?,
            bank: soundfont.options.bank,
            preset: soundfont.options.preset,
            use_effects: soundfont.options.use_effects,
            interpolator: match soundfont.options.interpolator {
                Interpolator::Nearest => 0,
                Interpolator::Linear => 1,
            },
            attack_curve: curve_id(soundfont.options.vol_envelope_options.attack_curve),
            decay_curve: curve_id(soundfont.options.vol_envelope_options.decay_curve),
            release_curve: curve_id(soundfont.options.vol_envelope_options.release_curve),
            sample_rate: params.sample_rate,
            channels: params.channels.count(),
        })
    }
}

fn curve_id(curve: EnvelopeCurveType) -> u8 {
    match curve {
        EnvelopeCurveType::Linear => 0,
        EnvelopeCurveType::Exponential => 1,
    }
}

pub fn resolve_default_soundfont_path() -> io::Result<PathBuf> {
    if let Some(path) = std::env::var_os("MERIDIAN_SOUNDFONT") {
        return Ok(PathBuf::from(path));
    }

    if let Some(path) = EMBEDDED_DEFAULT_SOUNDFONT_PATH.get() {
        return Ok(path.clone());
    }

    let directory = std::env::temp_dir()
        .join("meridian")
        .join("embedded-soundfonts");
    fs::create_dir_all(&directory)?;
    let output = directory.join(EMBEDDED_DEFAULT_SOUNDFONT_NAME);

    let needs_write = match fs::metadata(&output) {
        Ok(metadata) => metadata.len() != EMBEDDED_DEFAULT_SOUNDFONT_BYTES.len() as u64,
        Err(_) => true,
    };

    if needs_write {
        let temp = directory.join(format!(
            ".{}.{}.tmp",
            EMBEDDED_DEFAULT_SOUNDFONT_NAME,
            std::process::id()
        ));
        fs::write(&temp, EMBEDDED_DEFAULT_SOUNDFONT_BYTES)?;
        if fs::rename(&temp, &output).is_err() {
            if !output.exists() {
                fs::copy(&temp, &output)?;
            }
            let _ = fs::remove_file(&temp);
        }
    }

    let _ = EMBEDDED_DEFAULT_SOUNDFONT_PATH.set(output.clone());
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_soundfont_uses_embedded_asset() {
        let soundfont = MeridianSoundfont::default();
        assert!(soundfont.uses_default());
        let path = soundfont
            .resolved_path()
            .expect("embedded default soundfont should materialize");
        assert!(path.is_file());
        let len = fs::metadata(path)
            .expect("embedded default soundfont metadata")
            .len();
        assert_eq!(len, EMBEDDED_DEFAULT_SOUNDFONT_BYTES.len() as u64);
    }
}
