use std::sync::{Arc, Mutex};

use crate::error::MeridianError;

use super::{MIDIFileUnion, audio_cache::InRamAudioCache, parsed::ParsedMidiFile, ram::InRamMidiCache};

pub struct MidiCacheStack {
    parsed: Arc<ParsedMidiFile>,
    in_ram: Mutex<Option<Arc<InRamMidiCache>>>,
    audio: Mutex<Option<Arc<InRamAudioCache>>>,
}

impl Clone for MidiCacheStack {
    fn clone(&self) -> Self {
        let in_ram = self.in_ram.lock().ok().and_then(|cache| cache.clone());
        let audio = self.audio.lock().ok().and_then(|cache| cache.clone());
        Self {
            parsed: Arc::clone(&self.parsed),
            in_ram: Mutex::new(in_ram),
            audio: Mutex::new(audio),
        }
    }
}

impl MidiCacheStack {
    pub fn load(path: impl Into<std::path::PathBuf>) -> Result<Self, MeridianError> {
        Ok(Self {
            parsed: Arc::new(ParsedMidiFile::load_from_file(path)?),
            in_ram: Mutex::new(None),
            audio: Mutex::new(None),
        })
    }

    pub fn parsed(&self) -> &Arc<ParsedMidiFile> {
        &self.parsed
    }

    pub fn in_ram_cache(&self) -> Result<Arc<InRamMidiCache>, MeridianError> {
        let mut in_ram = self
            .in_ram
            .lock()
            .map_err(|_| MeridianError::InvalidMidi("midi cache lock poisoned".into()))?;
        if let Some(cache) = &*in_ram {
            return Ok(Arc::clone(cache));
        }

        let cache = Arc::new(InRamMidiCache::from_parsed(self.parsed())?);
        *in_ram = Some(Arc::clone(&cache));
        Ok(cache)
    }

    pub fn instantiate_in_ram(&self) -> Result<MIDIFileUnion, MeridianError> {
        Ok(MIDIFileUnion::InRam(self.in_ram_cache()?.instantiate()))
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
