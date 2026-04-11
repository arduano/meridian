use std::sync::{Arc, Mutex};

use crate::error::MeridianError;

use super::{
    analysis::{
        build_cached_midi_analysis_with_detailed_progress, AnalysisProgressUpdate,
        CachedMidiAnalysis,
    },
    audio_cache::InRamAudioCache,
    display_cache::DisplayMidiCache,
    materialized::{build_materialized_midi_with_progress_cancelable, MaterializeOptions},
    parsed::ParsedMidiFile,
    MIDIFileUnion, MidiBuildProgress,
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
        if let Some(cache) = load_cached_arc(&self.display, "display cache lock poisoned")? {
            return Ok(cache);
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
        store_cached_arc(
            &self.display,
            Arc::new(built_display),
            "display cache lock poisoned",
        )
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
        if let Some(cache) = load_cached_arc(&self.analysis, "analysis cache lock poisoned")? {
            return Ok(cache);
        }

        let cache = Arc::new(build_cached_midi_analysis_with_detailed_progress(
            self.parsed(),
            progress,
        )?);
        store_cached_arc(&self.analysis, cache, "analysis cache lock poisoned")
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
        if let Some(cache) = load_cached_arc(&self.audio, "audio cache lock poisoned")? {
            return Ok(cache);
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
        store_cached_arc(&self.audio, cache, "audio cache lock poisoned")
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
        let display_cache = load_cached_arc(&self.display, "display cache lock poisoned")?;
        let audio_cache = load_cached_arc(&self.audio, "audio cache lock poisoned")?;

        if let (Some(display_cache), Some(audio_cache)) = (&display_cache, &audio_cache) {
            return Ok((Arc::clone(display_cache), Arc::clone(audio_cache)));
        }

        let materialized = build_materialized_midi_with_progress_cancelable(
            self.parsed(),
            MaterializeOptions {
                display: display_cache.is_none(),
                audio: audio_cache.is_none(),
            },
            progress,
            should_cancel,
        )?;

        let display = match display_cache {
            Some(cache) => cache,
            None => store_cached_arc(
                &self.display,
                Arc::new(
                    materialized
                        .display
                        .expect("display cache must exist when requested"),
                ),
                "display cache lock poisoned",
            )?,
        };
        let audio = match audio_cache {
            Some(cache) => cache,
            None => store_cached_arc(
                &self.audio,
                Arc::new(
                    materialized
                        .audio
                        .expect("audio cache must exist when requested"),
                ),
                "audio cache lock poisoned",
            )?,
        };

        Ok((display, audio))
    }
}

fn load_cached_arc<T>(
    cache: &Mutex<Option<Arc<T>>>,
    poisoned_message: &'static str,
) -> Result<Option<Arc<T>>, MeridianError> {
    let cache = cache
        .lock()
        .map_err(|_| MeridianError::InvalidMidi(poisoned_message.into()))?;
    Ok(cache.as_ref().map(Arc::clone))
}

fn store_cached_arc<T>(
    cache: &Mutex<Option<Arc<T>>>,
    built: Arc<T>,
    poisoned_message: &'static str,
) -> Result<Arc<T>, MeridianError> {
    let mut cache = cache
        .lock()
        .map_err(|_| MeridianError::InvalidMidi(poisoned_message.into()))?;
    if let Some(existing) = &*cache {
        Ok(Arc::clone(existing))
    } else {
        *cache = Some(Arc::clone(&built));
        Ok(built)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicBool, Ordering};

    use super::MidiCacheStack;
    use crate::midi::test_support::{note_off, note_on, tempo, write_toolkit_midi, TestDir};

    fn test_cache_stack(label: &str) -> (TestDir, MidiCacheStack) {
        let dir = TestDir::new(label);
        let input = dir.path("input.mid");
        write_toolkit_midi(
            &input,
            96,
            &[vec![
                tempo(0, 500_000),
                note_on(0, 0, 60, 100),
                note_off(96, 0, 60),
                note_on(0, 0, 64, 100),
                note_off(96, 0, 64),
            ]],
        );
        let stack = MidiCacheStack::load(input).expect("load midi cache stack");
        (dir, stack)
    }

    #[test]
    fn display_progress_runs_without_holding_display_lock() {
        let (_dir, stack) = test_cache_stack("display-lock-scope");
        let saw_progress = AtomicBool::new(false);

        stack
            .display_cache_with_progress_cancelable(
                |_| {
                    saw_progress.store(true, Ordering::SeqCst);
                    assert!(
                        stack.display.try_lock().is_ok(),
                        "display mutex should be unlocked during progress callbacks"
                    );
                },
                || false,
            )
            .expect("build display cache");

        assert!(saw_progress.load(Ordering::SeqCst));
    }

    #[test]
    fn analysis_progress_runs_without_holding_analysis_lock() {
        let (_dir, stack) = test_cache_stack("analysis-lock-scope");
        let saw_progress = AtomicBool::new(false);

        stack
            .analysis_cache_with_detailed_progress(|_| {
                saw_progress.store(true, Ordering::SeqCst);
                assert!(
                    stack.analysis.try_lock().is_ok(),
                    "analysis mutex should be unlocked during progress callbacks"
                );
            })
            .expect("build analysis cache");

        assert!(saw_progress.load(Ordering::SeqCst));
    }

    #[test]
    fn render_progress_runs_without_holding_display_or_audio_locks() {
        let (_dir, stack) = test_cache_stack("render-lock-scope");
        let saw_progress = AtomicBool::new(false);

        stack
            .render_caches_with_progress_cancelable(
                |_| {
                    saw_progress.store(true, Ordering::SeqCst);
                    assert!(
                        stack.display.try_lock().is_ok(),
                        "display mutex should be unlocked during render progress callbacks"
                    );
                    assert!(
                        stack.audio.try_lock().is_ok(),
                        "audio mutex should be unlocked during render progress callbacks"
                    );
                },
                || false,
            )
            .expect("build render caches");

        assert!(saw_progress.load(Ordering::SeqCst));
    }
}
