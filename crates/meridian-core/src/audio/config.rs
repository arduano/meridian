use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use xsynth_core::soundfont::{EnvelopeCurveType, Interpolator, SoundfontInitOptions};
use xsynth_realtime::XSynthRealtimeConfig;

pub const DEFAULT_SOUNDFONT: &str =
    "/mnt/fat/Midis/Soundfonts/Loud and Proud Remastered/Axley Presets/Loud and Proud Remastered.sfz";

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
            path: PathBuf::from(DEFAULT_SOUNDFONT),
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
}

impl Default for XSynthSettings {
    fn default() -> Self {
        Self {
            config: Default::default(),
            limit_layers: true,
            layers: 4,
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
            backend: AudioBackend::Xsynth,
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
