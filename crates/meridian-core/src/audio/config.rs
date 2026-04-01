use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use xsynth_core::{
    AudioStreamParams, ChannelCount,
    channel_group::{ParallelismOptions, ThreadCount},
    soundfont::{EnvelopeCurveType, Interpolator, SoundfontInitOptions},
};
use xsynth_realtime::XSynthRealtimeConfig;

pub const DEFAULT_SOUNDFONT: &str =
    "assets/soundfonts/freepats-upright-kw-small/UprightPianoKW-small-20190703.sfz";

fn default_soundfont_path() -> PathBuf {
    if let Some(path) = std::env::var_os("MERIDIAN_SOUNDFONT") {
        return PathBuf::from(path);
    }

    let relative = PathBuf::from(DEFAULT_SOUNDFONT);
    if relative.exists() {
        return relative;
    }

    if let Ok(exe) = std::env::current_exe() {
        let exe_assets = exe
            .parent()
            .map(|parent| parent.join(DEFAULT_SOUNDFONT))
            .unwrap_or_else(|| relative.clone());
        if exe_assets.exists() {
            return exe_assets;
        }
    }

    PathBuf::from(DEFAULT_SOUNDFONT)
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AudioBackend {
    None,
    Xsynth,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct MeridianSoundfont {
    pub path: PathBuf,
    pub enabled: bool,
    pub options: SoundfontInitOptions,
}

impl Default for MeridianSoundfont {
    fn default() -> Self {
        Self {
            path: default_soundfont_path(),
            enabled: true,
            options: Default::default(),
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
            config: Default::default(),
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
            use_limiter: false,
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
    ) -> Self {
        Self {
            path: soundfont.path.clone(),
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
        }
    }
}

fn curve_id(curve: EnvelopeCurveType) -> u8 {
    match curve {
        EnvelopeCurveType::Linear => 0,
        EnvelopeCurveType::Exponential => 1,
    }
}
