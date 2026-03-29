use std::sync::{Arc, Mutex};

use crate::error::MeridianError;

use super::{
    MIDIFileUnion, audio_cache::InRamAudioCache, display_cache::DisplayMidiCache,
    parsed::ParsedMidiFile,
};

pub struct MidiCacheStack {
    parsed: Arc<ParsedMidiFile>,
    display: Mutex<Option<Arc<DisplayMidiCache>>>,
    audio: Mutex<Option<Arc<InRamAudioCache>>>,
}

impl Clone for MidiCacheStack {
    fn clone(&self) -> Self {
        let display = self.display.lock().ok().and_then(|cache| cache.clone());
        let audio = self.audio.lock().ok().and_then(|cache| cache.clone());
        Self {
            parsed: Arc::clone(&self.parsed),
            display: Mutex::new(display),
            audio: Mutex::new(audio),
        }
    }
}

impl MidiCacheStack {
    pub fn load(path: impl Into<std::path::PathBuf>) -> Result<Self, MeridianError> {
        Ok(Self {
            parsed: Arc::new(ParsedMidiFile::load_from_file(path)?),
            display: Mutex::new(None),
            audio: Mutex::new(None),
        })
    }

    pub fn parsed(&self) -> &Arc<ParsedMidiFile> {
        &self.parsed
    }

    pub fn display_cache(&self) -> Result<Arc<DisplayMidiCache>, MeridianError> {
        let mut display = self
            .display
            .lock()
            .map_err(|_| MeridianError::InvalidMidi("display cache lock poisoned".into()))?;
        if let Some(cache) = &*display {
            return Ok(Arc::clone(cache));
        }

        let cache = Arc::new(DisplayMidiCache::from_parsed(self.parsed())?);
        *display = Some(Arc::clone(&cache));
        Ok(cache)
    }

    pub fn instantiate_display_in_ram(&self) -> Result<MIDIFileUnion, MeridianError> {
        Ok(MIDIFileUnion::InRam(self.display_cache()?.instantiate()))
    }

    pub fn in_ram_cache(&self) -> Result<Arc<DisplayMidiCache>, MeridianError> {
        self.display_cache()
    }

    pub fn instantiate_in_ram(&self) -> Result<MIDIFileUnion, MeridianError> {
        self.instantiate_display_in_ram()
    }

    pub fn audio_cache(&self) -> Result<Arc<InRamAudioCache>, MeridianError> {
        let mut audio = self
            .audio
            .lock()
            .map_err(|_| MeridianError::InvalidMidi("audio cache lock poisoned".into()))?;
        if let Some(cache) = &*audio {
            return Ok(Arc::clone(cache));
        }

        let cache = Arc::new(InRamAudioCache::from_parsed(self.parsed())?);
        *audio = Some(Arc::clone(&cache));
        Ok(cache)
    }
}
