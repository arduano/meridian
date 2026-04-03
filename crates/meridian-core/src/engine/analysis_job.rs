use std::thread;

use crate::{
    midi::analysis::{
        MidiAnalysisKind, analyze_cached_midi, build_buckets_from_parsed_with_progress,
        select_analysis_kinds,
    },
    protocol::{
        AnalysisJobId, CoreErrorCode, CoreEvent, DisplayCacheId, MidiAnalysisJobEvent,
        MidiAnalysisJobStatus, ParsedMidiId,
    },
};

use super::{core_state::CoreState, support::error_event};

impl CoreState {
    pub(super) fn start_midi_analysis_job(
        &mut self,
        parsed_midi_id: ParsedMidiId,
        display_cache_id: Option<DisplayCacheId>,
        kinds: Vec<MidiAnalysisKind>,
        bucket_count: Option<usize>,
    ) -> Vec<CoreEvent> {
        let Some(parsed) = self.parsed_midis.get(&parsed_midi_id) else {
            return vec![error_event(
                CoreErrorCode::InvalidCommand,
                format!("unknown parsed_midi_id {}", parsed_midi_id.0),
            )];
        };

        if let Some(display_cache_id) = display_cache_id {
            let Some(resource) = self.display_caches.get(&display_cache_id) else {
                return vec![error_event(
                    CoreErrorCode::InvalidCommand,
                    format!("unknown display_cache_id {}", display_cache_id.0),
                )];
            };
            if resource.parsed_midi_id != parsed_midi_id {
                return vec![error_event(
                    CoreErrorCode::InvalidCommand,
                    "display cache does not belong to parsed midi",
                )];
            }
        }

        let job_id = AnalysisJobId(self.next_resource_id);
        self.next_resource_id += 1;
        let status = MidiAnalysisJobStatus::Running {
            job_id,
            parsed_midi_id,
            display_cache_id,
            kinds: kinds.clone(),
            progress: 0.0,
            status: "queued".into(),
        };
        self.analysis_jobs.insert(job_id, status.clone());

        let core_handle = self.core_handle.clone();
        let cache_stack = parsed.cache_stack.clone();
        thread::spawn(move || {
            let _ = core_handle.publish_analysis_job_event(MidiAnalysisJobEvent::Started {
                job_id,
                parsed_midi_id,
                display_cache_id,
                kinds: kinds.clone(),
            });

            let analysis = match cache_stack.analysis_cache_with_progress(|progress| {
                let _ = core_handle.publish_analysis_job_event(MidiAnalysisJobEvent::Progress {
                    job_id,
                    progress: progress.clamp(0.0, 0.95),
                    status: "building parsed analysis".into(),
                });
            }) {
                Ok(analysis) => analysis,
                Err(error) => {
                    let _ = core_handle.publish_analysis_job_event(MidiAnalysisJobEvent::Failed {
                        job_id,
                        message: error.to_string(),
                    });
                    return;
                }
            };

            let buckets = if kinds.is_empty() || kinds.contains(&MidiAnalysisKind::Buckets) {
                let buckets = build_buckets_from_parsed_with_progress(
                    cache_stack.parsed(),
                    bucket_count.unwrap_or(1024),
                    analysis.midi_length(),
                    |progress| {
                        let _ = core_handle.publish_analysis_job_event(
                            MidiAnalysisJobEvent::Progress {
                                job_id,
                                progress: (0.95 + progress.clamp(0.0, 1.0) * 0.05).clamp(0.95, 1.0),
                                status: "building bucket summary".into(),
                            },
                        );
                    },
                );
                match buckets {
                    Ok(buckets) => buckets,
                    Err(error) => {
                        let _ =
                            core_handle.publish_analysis_job_event(MidiAnalysisJobEvent::Failed {
                                job_id,
                                message: error.to_string(),
                            });
                        return;
                    }
                }
            } else {
                Vec::new()
            };

            let result = select_analysis_kinds(
                analyze_cached_midi(cache_stack.parsed(), analysis.as_ref(), buckets),
                &kinds,
            );
            let _ = core_handle
                .publish_analysis_job_event(MidiAnalysisJobEvent::Finished { job_id, result });
        });

        vec![CoreEvent::MidiAnalysisJobStatus { status }]
    }

    pub(super) fn get_midi_analysis_job_status(&self, job_id: AnalysisJobId) -> Vec<CoreEvent> {
        let Some(status) = self.analysis_jobs.get(&job_id) else {
            return vec![error_event(
                CoreErrorCode::InvalidCommand,
                format!("unknown analysis job {}", job_id.0),
            )];
        };
        vec![CoreEvent::MidiAnalysisJobStatus {
            status: status.clone(),
        }]
    }

    pub(super) fn handle_midi_analysis_job_update(&mut self, event: MidiAnalysisJobEvent) {
        match &event {
            MidiAnalysisJobEvent::Started {
                job_id,
                parsed_midi_id,
                display_cache_id,
                kinds,
            } => {
                self.analysis_jobs.insert(
                    *job_id,
                    MidiAnalysisJobStatus::Running {
                        job_id: *job_id,
                        parsed_midi_id: *parsed_midi_id,
                        display_cache_id: *display_cache_id,
                        kinds: kinds.clone(),
                        progress: 0.0,
                        status: "started".into(),
                    },
                );
            }
            MidiAnalysisJobEvent::Progress {
                job_id,
                progress,
                status,
            } => {
                if let Some(MidiAnalysisJobStatus::Running {
                    parsed_midi_id,
                    display_cache_id,
                    kinds,
                    ..
                }) = self.analysis_jobs.get(job_id).cloned()
                {
                    self.analysis_jobs.insert(
                        *job_id,
                        MidiAnalysisJobStatus::Running {
                            job_id: *job_id,
                            parsed_midi_id,
                            display_cache_id,
                            kinds,
                            progress: *progress,
                            status: status.clone(),
                        },
                    );
                }
            }
            MidiAnalysisJobEvent::Finished { job_id, result } => {
                if let Some(MidiAnalysisJobStatus::Running {
                    parsed_midi_id,
                    display_cache_id,
                    kinds,
                    ..
                }) = self.analysis_jobs.get(job_id).cloned()
                {
                    self.analysis_jobs.insert(
                        *job_id,
                        MidiAnalysisJobStatus::Finished {
                            job_id: *job_id,
                            parsed_midi_id,
                            display_cache_id,
                            kinds,
                            result: result.clone(),
                        },
                    );
                }
            }
            MidiAnalysisJobEvent::Failed { job_id, message } => {
                if let Some(MidiAnalysisJobStatus::Running {
                    parsed_midi_id,
                    display_cache_id,
                    kinds,
                    ..
                }) = self.analysis_jobs.get(job_id).cloned()
                {
                    self.analysis_jobs.insert(
                        *job_id,
                        MidiAnalysisJobStatus::Failed {
                            job_id: *job_id,
                            parsed_midi_id,
                            display_cache_id,
                            kinds,
                            message: message.clone(),
                        },
                    );
                }
            }
        }

        self.broadcast(CoreEvent::MidiAnalysisJob {
            event: event.clone(),
        });
        if let Some(status) = match &event {
            MidiAnalysisJobEvent::Started { job_id, .. }
            | MidiAnalysisJobEvent::Progress { job_id, .. }
            | MidiAnalysisJobEvent::Finished { job_id, .. }
            | MidiAnalysisJobEvent::Failed { job_id, .. } => self.analysis_jobs.get(job_id),
        } {
            self.broadcast(CoreEvent::MidiAnalysisJobStatus {
                status: status.clone(),
            });
        }
    }
}
