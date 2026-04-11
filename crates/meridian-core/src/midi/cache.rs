use std::sync::{Arc, Mutex};

use crate::error::MeridianError;

use super::{
    MIDIFileUnion, MidiBuildProgress,
    analysis::{
        AnalysisProgressUpdate, CachedMidiAnalysis,
        build_cached_midi_analysis_with_detailed_progress,
    },
    audio_cache::InRamAudioCache,
    display_cache::DisplayMidiCache,
    materialized::{MaterializeOptions, build_materialized_midi_with_progress_cancelable},
    parsed::ParsedMidiFile,
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
        Self::load_with_progress_cancelable(path, progress, || false)
    }

    pub fn load_with_progress_cancelable(
        path: impl Into<std::path::PathBuf>,
        progress: impl FnMut(f32),
        should_cancel: impl Fn() -> bool,
    ) -> Result<Self, MeridianError> {
        Ok(Self {
            parsed: Arc::new(ParsedMidiFile::load_from_file_with_progress_cancelable(
                path,
                progress,
                should_cancel,
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
        progress: impl FnMut(MidiBuildProgress),
    ) -> Result<Arc<DisplayMidiCache>, MeridianError> {
        self.display_cache_with_progress_cancelable(progress, || false)
    }

    pub fn display_cache_with_progress_cancelable(
        &self,
        progress: impl FnMut(MidiBuildProgress),
        should_cancel: impl Fn() -> bool,
    ) -> Result<Arc<DisplayMidiCache>, MeridianError> {
        let mut display = self
            .display
            .lock()
            .map_err(|_| MeridianError::InvalidMidi("display cache lock poisoned".into()))?;
        if let Some(cache) = &*display {
            return Ok(Arc::clone(cache));
        }

        let materialized = build_materialized_midi_with_progress_cancelable(
            self.parsed(),
            MaterializeOptions {
                display: true,
                audio: false,
            },
            progress,
            should_cancel,
        )?;
        let built_display = materialized
            .display
            .expect("display cache must exist when display materialization is requested");
        let display_arc = Arc::new(built_display);
        *display = Some(Arc::clone(&display_arc));
        Ok(display_arc)
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
        progress: impl FnMut(f32) + Send,
    ) -> Result<Arc<CachedMidiAnalysis>, MeridianError> {
        let progress = Mutex::new(progress);
        self.analysis_cache_with_detailed_progress(move |update| {
            if let Ok(mut callback) = progress.lock() {
                (*callback)(update.progress);
            }
        })
    }

    pub fn analysis_cache_with_detailed_progress(
        &self,
        progress: impl Fn(AnalysisProgressUpdate) + Sync + Send,
    ) -> Result<Arc<CachedMidiAnalysis>, MeridianError> {
        let mut analysis = self
            .analysis
            .lock()
            .map_err(|_| MeridianError::InvalidMidi("analysis cache lock poisoned".into()))?;
        if let Some(cache) = &*analysis {
            return Ok(Arc::clone(cache));
        }

        let cache = Arc::new(build_cached_midi_analysis_with_detailed_progress(
            self.parsed(),
            progress,
        )?);
        *analysis = Some(Arc::clone(&cache));
        Ok(cache)
    }

    pub fn audio_cache(&self) -> Result<Arc<InRamAudioCache>, MeridianError> {
        self.audio_cache_with_progress(|_| {})
    }

    pub fn audio_cache_with_progress(
        &self,
        progress: impl FnMut(MidiBuildProgress),
    ) -> Result<Arc<InRamAudioCache>, MeridianError> {
        self.audio_cache_with_progress_cancelable(progress, || false)
    }

    pub fn audio_cache_with_progress_cancelable(
        &self,
        progress: impl FnMut(MidiBuildProgress),
        should_cancel: impl Fn() -> bool,
    ) -> Result<Arc<InRamAudioCache>, MeridianError> {
        let mut audio = self
            .audio
            .lock()
            .map_err(|_| MeridianError::InvalidMidi("audio cache lock poisoned".into()))?;
        if let Some(cache) = &*audio {
            return Ok(Arc::clone(cache));
        }

        let materialized = build_materialized_midi_with_progress_cancelable(
            self.parsed(),
            MaterializeOptions {
                display: false,
                audio: true,
            },
            progress,
            should_cancel,
        )?;
        let cache = Arc::new(
            materialized
                .audio
                .expect("audio cache must exist when audio materialization is requested"),
        );
        *audio = Some(Arc::clone(&cache));
        Ok(cache)
    }

    pub fn render_caches_with_progress(
        &self,
        progress: impl FnMut(MidiBuildProgress),
    ) -> Result<(Arc<DisplayMidiCache>, Arc<InRamAudioCache>), MeridianError> {
        self.render_caches_with_progress_cancelable(progress, || false)
    }

    pub fn render_caches_with_progress_cancelable(
        &self,
        progress: impl FnMut(MidiBuildProgress),
        should_cancel: impl Fn() -> bool,
    ) -> Result<(Arc<DisplayMidiCache>, Arc<InRamAudioCache>), MeridianError> {
        let mut display = self
            .display
            .lock()
            .map_err(|_| MeridianError::InvalidMidi("display cache lock poisoned".into()))?;
        let mut audio = self
            .audio
            .lock()
            .map_err(|_| MeridianError::InvalidMidi("audio cache lock poisoned".into()))?;

        if let (Some(display_cache), Some(audio_cache)) = (&*display, &*audio) {
            return Ok((Arc::clone(display_cache), Arc::clone(audio_cache)));
        }

        let materialized = build_materialized_midi_with_progress_cancelable(
            self.parsed(),
            MaterializeOptions {
                display: display.is_none(),
                audio: audio.is_none(),
            },
            progress,
            should_cancel,
        )?;

        if display.is_none() {
            *display = Some(Arc::new(
                materialized
                    .display
                    .expect("display cache must exist when requested"),
            ));
        }
        if audio.is_none() {
            *audio = Some(Arc::new(
                materialized
                    .audio
                    .expect("audio cache must exist when requested"),
            ));
        }
        Ok((
            Arc::clone(display.as_ref().expect("display cache must be initialized")),
            Arc::clone(audio.as_ref().expect("audio cache must be initialized")),
        ))
    }
}
