use crate::{
    engine::support::error_event,
    midi::analysis::{analyze_midi, analyze_parsed_midi_with_progress},
    protocol::{CoreErrorCode, CoreEvent, ProcessedMidiId},
};

use super::super::core_state::CoreState;

impl CoreState {
    pub(in crate::engine) fn analyze_active_midi(
        &self,
        bucket_count: Option<usize>,
    ) -> Vec<CoreEvent> {
        let bucket_count = bucket_count.unwrap_or(1024);
        if let Some(processed_midi_id) = self.active_processed_midi_id {
            let Some(resource) = self.processed_midis.get(&processed_midi_id) else {
                return vec![error_event(
                    CoreErrorCode::Internal,
                    "active processed MIDI is missing from registry",
                )];
            };
            let Some(parsed) = self.parsed_midis.get(&resource.parsed_midi_id) else {
                return vec![error_event(
                    CoreErrorCode::Internal,
                    "active processed MIDI is missing its parsed MIDI parent",
                )];
            };
            return vec![CoreEvent::MidiAnalysis {
                processed_midi_id: Some(processed_midi_id),
                display_cache_id: None,
                analysis: analyze_midi(
                    parsed.cache_stack.parsed(),
                    resource.midi.display_cache().as_ref(),
                    resource.midi.analysis_cache().as_ref(),
                    bucket_count,
                ),
            }];
        }

        if let Some(display_cache_id) = self.active_display_cache_id {
            let Some(resource) = self.display_caches.get(&display_cache_id) else {
                return vec![error_event(
                    CoreErrorCode::Internal,
                    "active display cache is missing from registry",
                )];
            };
            let Some(parsed) = self.parsed_midis.get(&resource.parsed_midi_id) else {
                return vec![error_event(
                    CoreErrorCode::Internal,
                    "active display cache is missing its parsed MIDI parent",
                )];
            };
            let Ok(analysis) = parsed.cache_stack.analysis_cache() else {
                return vec![error_event(
                    CoreErrorCode::Internal,
                    "failed to build analysis cache",
                )];
            };
            let analysis = match analyze_parsed_midi_with_progress(
                parsed.cache_stack.parsed(),
                analysis.as_ref(),
                bucket_count,
                |_| {},
            ) {
                Ok(analysis) => analysis,
                Err(error) => {
                    return vec![error_event(CoreErrorCode::Internal, error.to_string())];
                }
            };
            return vec![CoreEvent::MidiAnalysis {
                processed_midi_id: None,
                display_cache_id: Some(display_cache_id),
                analysis,
            }];
        }

        if let Some(parsed_midi_id) = self.active_parsed_midi_id {
            let Some(parsed) = self.parsed_midis.get(&parsed_midi_id) else {
                return vec![error_event(
                    CoreErrorCode::Internal,
                    "active parsed MIDI is missing from registry",
                )];
            };
            let Ok(analysis) = parsed.cache_stack.analysis_cache() else {
                return vec![error_event(
                    CoreErrorCode::Internal,
                    "failed to build analysis cache",
                )];
            };
            let analysis = match analyze_parsed_midi_with_progress(
                parsed.cache_stack.parsed(),
                analysis.as_ref(),
                bucket_count,
                |_| {},
            ) {
                Ok(analysis) => analysis,
                Err(error) => {
                    return vec![error_event(CoreErrorCode::Internal, error.to_string())];
                }
            };
            return vec![CoreEvent::MidiAnalysis {
                processed_midi_id: None,
                display_cache_id: None,
                analysis,
            }];
        }

        vec![error_event(
            CoreErrorCode::NoMidiLoaded,
            "no active midi available for analysis",
        )]
    }

    pub(in crate::engine) fn analyze_processed_midi(
        &self,
        processed_midi_id: ProcessedMidiId,
        bucket_count: Option<usize>,
    ) -> Vec<CoreEvent> {
        let Some(resource) = self.processed_midis.get(&processed_midi_id) else {
            return vec![error_event(
                CoreErrorCode::InvalidCommand,
                format!("unknown processed_midi_id {}", processed_midi_id.0),
            )];
        };
        let Some(parsed) = self.parsed_midis.get(&resource.parsed_midi_id) else {
            return vec![error_event(
                CoreErrorCode::Internal,
                "processed midi is missing its parsed MIDI parent",
            )];
        };
        vec![CoreEvent::MidiAnalysis {
            processed_midi_id: Some(processed_midi_id),
            display_cache_id: None,
            analysis: analyze_midi(
                parsed.cache_stack.parsed(),
                resource.midi.display_cache().as_ref(),
                resource.midi.analysis_cache().as_ref(),
                bucket_count.unwrap_or(1024),
            ),
        }]
    }
}
