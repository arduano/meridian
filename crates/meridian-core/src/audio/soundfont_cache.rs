use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use xsynth_core::{
    AudioStreamParams,
    soundfont::{SampleSoundfont, SoundfontBase},
};

use crate::error::MeridianError;

use super::config::{MeridianSoundfont, SoundfontCacheKey};

pub struct SoundfontCache {
    loaded: Mutex<HashMap<SoundfontCacheKey, Arc<dyn SoundfontBase>>>,
}

impl Default for SoundfontCache {
    fn default() -> Self {
        Self::new()
    }
}

impl SoundfontCache {
    pub fn new() -> Self {
        Self {
            loaded: Mutex::new(HashMap::new()),
        }
    }

    pub fn load_enabled(
        &self,
        soundfonts: &[MeridianSoundfont],
        params: AudioStreamParams,
    ) -> Result<Vec<Arc<dyn SoundfontBase>>, MeridianError> {
        let mut loaded = self
            .loaded
            .lock()
            .map_err(|_| MeridianError::InvalidMidi("soundfont cache lock poisoned".into()))?;
        let mut out = Vec::new();
        for sf in soundfonts.iter().rev().filter(|sf| sf.enabled) {
            let key = SoundfontCacheKey::new(sf, params)
                .map_err(|e| MeridianError::MidiLoad(format!("soundfont resolve failed: {e}")))?;
            if let Some(existing) = loaded.get(&key) {
                out.push(Arc::clone(existing));
                continue;
            }

            let path = sf
                .resolved_path()
                .map_err(|e| MeridianError::MidiLoad(format!("soundfont resolve failed: {e}")))?;
            let soundfont = SampleSoundfont::new(&path, params, sf.options)
                .map_err(|e| MeridianError::MidiLoad(format!("soundfont load failed: {e:?}")))?;
            let soundfont: Arc<dyn SoundfontBase> = Arc::new(soundfont);
            loaded.insert(key, Arc::clone(&soundfont));
            out.push(soundfont);
        }
        Ok(out)
    }
}
