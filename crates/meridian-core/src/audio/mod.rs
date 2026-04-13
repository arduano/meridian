mod clock;
mod config;
mod playback;
mod player;
mod render;
mod soundfont_cache;
mod xsynth;

pub use clock::PlaybackClock;
pub use config::{
    AudioBackend, AudioConfig, EMBEDDED_DEFAULT_SOUNDFONT_ASSET, EMBEDDED_DEFAULT_SOUNDFONT_NAME,
    MeridianSoundfont, XSynthRenderSettings, XSynthSettings, resolve_default_soundfont_path,
};
pub use playback::LiveAudioSession;
pub use player::{AudioStatus, MeridianAudioPlayer};
pub use render::{
    AudioRenderConfig, AudioRenderEvent, render_audio, render_audio_from_cache, render_audio_to_wav,
};
#[cfg(unix)]
pub(crate) use render::{render_audio_pipe_from_cache, resolve_video_audio_settings};
pub use soundfont_cache::SoundfontCache;
pub use xsynth_core::{
    ChannelCount,
    channel_group::ThreadCount,
    soundfont::{EnvelopeCurveType, Interpolator},
};
