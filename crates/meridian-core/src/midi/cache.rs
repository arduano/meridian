use std::sync::{Arc, Mutex};

use crate::error::MeridianError;

use super::{
    MIDIFileUnion, analysis::CachedMidiAnalysis, audio_cache::InRamAudioCache,
    display_cache::DisplayMidiCache, parsed::ParsedMidiFile,
};

pub struct MidiCacheStack {
    parsed: Arc<ParsedMidiFile>,
    display: Mutex<Option<Arc<DisplayMidiCache>>>,
    analysis: Mutex<Option<Arc<CachedMidiAnalysis>>>,
    audio: Mutex<Option<Arc<InRamAudioCache>>>,
}

impl Clone for MidiCacheStack {
    fn clone(&self) -> Self {
        let display = self.display.lock().ok().and_then(|cache| cache.clone());
        let analysis = self.analysis.lock().ok().and_then(|cache| cache.clone());
        let audio = self.audio.lock().ok().and_then(|cache| cache.clone());
        Self {
            parsed: Arc::clone(&self.parsed),
            display: Mutex::new(display),
            analysis: Mutex::new(analysis),
            audio: Mutex::new(audio),
        }
    }
}

impl MidiCacheStack {
    pub fn load(path: impl Into<std::path::PathBuf>) -> Result<Self, MeridianError> {
        Self::load_with_progress(path, |_| {})
    }

    pub fn load_with_progress(
        path: impl Into<std::path::PathBuf>,
        progress: impl FnMut(f32),
    ) -> Result<Self, MeridianError> {
        Ok(Self {
            parsed: Arc::new(ParsedMidiFile::load_from_file_with_progress(
                path, progress,
            )?),
            display: Mutex::new(None),
            analysis: Mutex::new(None),
            audio: Mutex::new(None),
        })
    }

    pub fn parsed(&self) -> &Arc<ParsedMidiFile> {
        &self.parsed
    }

    pub fn display_cache(&self) -> Result<Arc<DisplayMidiCache>, MeridianError> {
        self.display_cache_with_progress(|_| {})
    }

    pub fn display_cache_with_progress(
        &self,
        progress: impl FnMut(f32),
    ) -> Result<Arc<DisplayMidiCache>, MeridianError> {
        Ok(self.display_and_analysis_with_progress(progress)?.0)
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

    pub fn analysis_cache(&self) -> Result<Arc<CachedMidiAnalysis>, MeridianError> {
        self.analysis_cache_with_progress(|_| {})
    }

    pub fn analysis_cache_with_progress(
        &self,
        progress: impl FnMut(f32),
    ) -> Result<Arc<CachedMidiAnalysis>, MeridianError> {
        Ok(self.display_and_analysis_with_progress(progress)?.1)
    }

    pub fn audio_cache(&self) -> Result<Arc<InRamAudioCache>, MeridianError> {
        self.audio_cache_with_progress(|_| {})
    }

    pub fn audio_cache_with_progress(
        &self,
        progress: impl FnMut(f32),
    ) -> Result<Arc<InRamAudioCache>, MeridianError> {
        let mut audio = self
            .audio
            .lock()
            .map_err(|_| MeridianError::InvalidMidi("audio cache lock poisoned".into()))?;
        if let Some(cache) = &*audio {
            return Ok(Arc::clone(cache));
        }

        let cache = Arc::new(InRamAudioCache::from_parsed_with_progress(
            self.parsed(),
            progress,
        )?);
        *audio = Some(Arc::clone(&cache));
        Ok(cache)
    }

    fn display_and_analysis_with_progress(
        &self,
        progress: impl FnMut(f32),
    ) -> Result<(Arc<DisplayMidiCache>, Arc<CachedMidiAnalysis>), MeridianError> {
        let mut display = self
            .display
            .lock()
            .map_err(|_| MeridianError::InvalidMidi("display cache lock poisoned".into()))?;
        let mut analysis = self
            .analysis
            .lock()
            .map_err(|_| MeridianError::InvalidMidi("analysis cache lock poisoned".into()))?;
        if let (Some(display), Some(analysis)) = (&*display, &*analysis) {
            return Ok((Arc::clone(display), Arc::clone(analysis)));
        }

        let (built_display, built_analysis) =
            super::ram::parse::build_in_ram_cache_with_progress(self.parsed(), progress)?;
        let display_arc = Arc::new(built_display);
        let analysis_arc = Arc::new(built_analysis);
        *display = Some(Arc::clone(&display_arc));
        *analysis = Some(Arc::clone(&analysis_arc));
        Ok((display_arc, analysis_arc))
    }
}
