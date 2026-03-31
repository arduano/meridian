mod clock;
mod config;
mod playback;
mod player;
mod render;
mod soundfont_cache;
mod xsynth;

pub use clock::PlaybackClock;
pub use config::{
    AudioBackend, AudioConfig, DEFAULT_SOUNDFONT, MeridianSoundfont, XSynthRenderSettings,
    XSynthSettings,
};
pub use playback::LiveAudioSession;
pub use player::{AudioStatus, MeridianAudioPlayer};
pub use render::{
    AudioRenderConfig, AudioRenderEvent, render_audio, render_audio_from_cache, render_audio_to_wav,
};
pub use soundfont_cache::SoundfontCache;
pub use xsynth_core::{
    ChannelCount,
    channel_group::ThreadCount,
    soundfont::{EnvelopeCurveType, Interpolator},
};
