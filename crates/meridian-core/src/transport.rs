use std::time::Instant;

pub const PREVIEW_START_TIME_SECONDS: f64 = -1.0;

#[derive(Debug, Clone, Copy, Default)]
pub struct TransportSnapshot {
    pub current_time: f64,
    pub playing: bool,
}

#[derive(Debug, Clone, Default)]
pub struct TransportState {
    current_time: f64,
    playing: bool,
    last_tick: Option<Instant>,
}

impl TransportState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn snapshot(&self) -> TransportSnapshot {
        TransportSnapshot {
            current_time: self.current_time,
            playing: self.playing,
        }
    }

    pub fn current_time(&self) -> f64 {
        self.current_time
    }

    pub fn playing(&self) -> bool {
        self.playing
    }

    pub fn reset(&mut self, now: Instant) {
        self.current_time = PREVIEW_START_TIME_SECONDS;
        self.playing = false;
        self.last_tick = Some(now);
    }

    pub fn set_time(&mut self, time: f64, midi_length: f64, now: Instant) {
        self.current_time = time.min(midi_length.max(0.0));
        self.last_tick = Some(now);
    }

    pub fn step_time(&mut self, delta: f64, midi_length: f64, now: Instant) {
        self.current_time =
            (self.current_time + delta).clamp(PREVIEW_START_TIME_SECONDS, midi_length.max(0.0));
        self.last_tick = Some(now);
    }

    pub fn set_playing(&mut self, playing: bool, now: Instant) {
        self.playing = playing;
        self.last_tick = Some(now);
    }

    pub fn toggle_playing(&mut self, now: Instant) {
        self.playing = !self.playing;
        self.last_tick = Some(now);
    }

    pub fn sync(&mut self, midi_length: f64, now: Instant) -> TransportSync {
        let mut stopped_at_end = false;
        if self.playing
            && let Some(last_tick) = self.last_tick {
                self.current_time += now.duration_since(last_tick).as_secs_f64();
                self.current_time = self.current_time.min(midi_length.max(0.0));
                if self.current_time >= midi_length && midi_length > 0.0 {
                    self.playing = false;
                    stopped_at_end = true;
                }
            }
        self.last_tick = Some(now);
        TransportSync {
            snapshot: self.snapshot(),
            stopped_at_end,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{PREVIEW_START_TIME_SECONDS, TransportState};
    use std::time::{Duration, Instant};

    #[test]
    fn reset_starts_preview_at_negative_one_second() {
        let mut transport = TransportState::new();
        transport.reset(Instant::now());
        assert_eq!(transport.current_time(), PREVIEW_START_TIME_SECONDS);
    }

    #[test]
    fn set_time_allows_negative_preroll_below_preview_floor() {
        let mut transport = TransportState::new();
        transport.set_time(-3.5, 8.0, Instant::now());
        assert_eq!(transport.current_time(), -3.5);
    }

    #[test]
    fn step_time_clamps_interactive_preview_to_negative_one_second() {
        let mut transport = TransportState::new();
        transport.reset(Instant::now());
        transport.step_time(-5.0, 8.0, Instant::now());
        assert_eq!(transport.current_time(), PREVIEW_START_TIME_SECONDS);
    }

    #[test]
    fn sync_counts_up_from_negative_preview_time() {
        let mut transport = TransportState::new();
        let start = Instant::now();
        transport.reset(start);
        transport.set_playing(true, start);
        let sync = transport.sync(8.0, start + Duration::from_millis(1500));
        assert!(sync.snapshot.current_time > 0.49);
        assert!(sync.snapshot.current_time < 0.51);
    }
}

#[derive(Debug, Clone, Copy)]
pub struct TransportSync {
    pub snapshot: TransportSnapshot,
    pub stopped_at_end: bool,
}
