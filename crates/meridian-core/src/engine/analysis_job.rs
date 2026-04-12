use std::thread;

use crate::{
    midi::analysis::{
        MidiAnalysisKind, analyze_cached_midi_with_gzip_bytes, select_analysis_kinds,
    },
    protocol::{
        AnalysisJobId, CoreErrorCode, CoreEvent, MidiAnalysisJobEvent, MidiAnalysisJobStatus,
        ParsedMidiId,
    },
};

use super::{core_state::CoreState, support::error_event};

impl CoreState {
    pub(super) fn start_midi_analysis_job(
        &mut self,
        parsed_midi_id: ParsedMidiId,
        kinds: Vec<MidiAnalysisKind>,
        bucket_count: Option<usize>,
    ) -> Vec<CoreEvent> {
        let Some(parsed) = self.parsed_midis.get(&parsed_midi_id) else {
            return vec![error_event(
                CoreErrorCode::InvalidCommand,
                format!("unknown parsed_midi_id {}", parsed_midi_id.0),
            )];
        };

        let job_id = AnalysisJobId(self.resource_ids.next_job_id());
        let status = MidiAnalysisJobStatus::Running {
            job_id,
            parsed_midi_id,
            kinds: kinds.clone(),
            progress: 0.0,
            status: "Queued".into(),
        };
        self.analysis_jobs.insert(job_id, status.clone());

        let core_handle = self.core_handle.clone();
        let cache_stack = parsed.cache_stack.clone();
        thread::spawn(move || {
            let gzip_handle = if kinds.is_empty() || kinds.contains(&MidiAnalysisKind::File) {
                let parsed = std::sync::Arc::clone(cache_stack.parsed());
                let path = parsed.signature().filepath.clone();
                Some(thread::spawn(move || {
                    parsed.cached_gzip_size().map_err(|error| {
                        format!(
                            "failed to compute gzip size for {}: {error}",
                            path.display()
                        )
                    })
                }))
            } else {
                None
            };

            let _ = core_handle.publish_analysis_job_event(MidiAnalysisJobEvent::Started {
                job_id,
                parsed_midi_id,
                kinds: kinds.clone(),
            });

            let analysis = match cache_stack.analysis_cache_with_detailed_progress(|update| {
                let _ = core_handle.publish_analysis_job_event(MidiAnalysisJobEvent::Progress {
                    job_id,
                    progress: update.progress.clamp(0.0, 0.95),
                    status: update.status,
                });
            }) {
                Ok(analysis) => analysis,
                Err(error) => {
                    let _ = join_gzip_metrics_worker(gzip_handle);
                    let _ = core_handle.publish_analysis_job_event(MidiAnalysisJobEvent::Failed {
                        job_id,
                        message: error.to_string(),
                    });
                    return;
                }
            };

            let buckets = if kinds.is_empty() || kinds.contains(&MidiAnalysisKind::Buckets) {
                let _ = core_handle.publish_analysis_job_event(MidiAnalysisJobEvent::Progress {
                    job_id,
                    progress: 0.975,
                    status: "Building Bucket Summary".into(),
                });
                analysis.build_buckets_from_note_spans(bucket_count.unwrap_or(1024))
            } else {
                Vec::new()
            };

            if gzip_handle.is_some() {
                let _ = core_handle.publish_analysis_job_event(MidiAnalysisJobEvent::Progress {
                    job_id,
                    progress: if buckets.is_empty() { 0.975 } else { 0.99 },
                    status: "Computing File Metrics".into(),
                });
            }
            let gzip_bytes = match join_gzip_metrics_worker(gzip_handle) {
                Ok(gzip_bytes) => gzip_bytes,
                Err(message) => {
                    let _ = core_handle.publish_analysis_job_event(MidiAnalysisJobEvent::Failed {
                        job_id,
                        message,
                    });
                    return;
                }
            };
            let result = select_analysis_kinds(
                analyze_cached_midi_with_gzip_bytes(
                    cache_stack.parsed(),
                    analysis.as_ref(),
                    buckets,
                    gzip_bytes,
                ),
                &kinds,
            );
            let _ = core_handle
                .publish_analysis_job_event(MidiAnalysisJobEvent::Finished { job_id, result });
        });

        vec![CoreEvent::MidiAnalysisJobStatus { status }]
    }

    pub(super) fn handle_midi_analysis_job_update(&mut self, event: MidiAnalysisJobEvent) {
        match &event {
            MidiAnalysisJobEvent::Started {
                job_id,
                parsed_midi_id,
                kinds,
            } => {
                self.analysis_jobs.insert(
                    *job_id,
                    MidiAnalysisJobStatus::Running {
                        job_id: *job_id,
                        parsed_midi_id: *parsed_midi_id,
                        kinds: kinds.clone(),
                        progress: 0.0,
                        status: "Started".into(),
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
                    kinds,
                    ..
                }) = self.analysis_jobs.get(job_id).cloned()
                {
                    self.analysis_jobs.insert(
                        *job_id,
                        MidiAnalysisJobStatus::Running {
                            job_id: *job_id,
                            parsed_midi_id,
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
                    kinds,
                    ..
                }) = self.analysis_jobs.get(job_id).cloned()
                {
                    self.analysis_jobs.insert(
                        *job_id,
                        MidiAnalysisJobStatus::Finished {
                            job_id: *job_id,
                            parsed_midi_id,
                            kinds,
                            result: result.clone(),
                        },
                    );
                }
            }
            MidiAnalysisJobEvent::Failed { job_id, message } => {
                if let Some(MidiAnalysisJobStatus::Running {
                    parsed_midi_id,
                    kinds,
                    ..
                }) = self.analysis_jobs.get(job_id).cloned()
                {
                    self.analysis_jobs.insert(
                        *job_id,
                        MidiAnalysisJobStatus::Failed {
                            job_id: *job_id,
                            parsed_midi_id,
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

fn join_gzip_metrics_worker(
    handle: Option<thread::JoinHandle<Result<u64, String>>>,
) -> Result<Option<u64>, String> {
    let Some(handle) = handle else {
        return Ok(None);
    };

    handle
        .join()
        .map_err(|_| "gzip metrics worker panicked".to_string())?
        .map(Some)
}
