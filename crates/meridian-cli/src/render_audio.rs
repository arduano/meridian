use std::{
    io::{self, BufWriter, Write},
    path::{Path, PathBuf},
};

use meridian_core::{
    MeridianError,
    audio::{
        AudioBackend, AudioConfig, AudioRenderConfig, MeridianSoundfont, SoundfontCache,
        render_audio_to_wav,
    },
    midi::MidiCacheStack,
};

pub fn run(
    midi: &Path,
    output: &Path,
    sample_rate: u32,
    channels: u16,
    use_limiter: bool,
    soundfonts: &[PathBuf],
) -> Result<(), MeridianError> {
    let midi_cache = MidiCacheStack::load(midi.to_path_buf())?;
    let mut audio_config = AudioConfig::default();
    audio_config.backend = AudioBackend::Xsynth;
    if !soundfonts.is_empty() {
        audio_config.soundfonts = soundfonts
            .iter()
            .cloned()
            .map(|path| MeridianSoundfont {
                path,
                ..MeridianSoundfont::default()
            })
            .collect();
    }
    audio_config.xsynth.render.audio_params.sample_rate = sample_rate;
    audio_config.xsynth.render.audio_params.channels = channels.into();
    audio_config.xsynth.render.use_limiter = use_limiter;

    let render_config = AudioRenderConfig {
        midi_path: Some(midi.to_path_buf()),
        audio: Some(audio_config.clone()),
        output: output.to_path_buf(),
        sample_rate: None,
        channels: None,
        use_limiter: None,
        soundfonts: Vec::new(),
    };
    let soundfont_cache = SoundfontCache::new();
    let mut stdout = BufWriter::new(io::stdout().lock());
    render_audio_to_wav(
        &midi_cache,
        &audio_config,
        &soundfont_cache,
        &render_config,
        |event| {
            let _ = write_event(&mut stdout, &event);
        },
    )
}

fn write_event(
    stdout: &mut BufWriter<impl Write>,
    event: &meridian_core::audio::AudioRenderEvent,
) -> Result<(), MeridianError> {
    serde_json::to_writer(&mut *stdout, event).map_err(|e| {
        MeridianError::Platform(format!("failed to serialize audio render event: {e}"))
    })?;
    stdout.write_all(b"\n")?;
    stdout.flush()?;
    Ok(())
}
