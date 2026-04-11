use std::{
    sync::{Condvar, Mutex},
    time::Instant,
};

pub enum WaitResult {
    Reached,
    StateChanged,
    Killed,
}

struct ClockState {
    base_time: f64,
    anchor: Instant,
    playing: bool,
    generation: u64,
    killed: bool,
}

pub struct PlaybackClock {
    state: Mutex<ClockState>,
    wake: Condvar,
}

impl PlaybackClock {
    pub fn new() -> Self {
        Self {
            state: Mutex::new(ClockState {
                base_time: 0.0,
                anchor: Instant::now(),
                playing: false,
                generation: 0,
                killed: false,
            }),
            wake: Condvar::new(),
        }
    }

    pub fn snapshot_time(&self) -> f64 {
        let state = self.state.lock().unwrap();
        current_time(&state)
    }

    pub fn snapshot_base_time(&self) -> f64 {
        let state = self.state.lock().unwrap();
        state.base_time
    }

    pub fn set_time(&self, time: f64) {
        let mut state = self.state.lock().unwrap();
        state.base_time = time.max(0.0);
        state.anchor = Instant::now();
        state.generation += 1;
        self.wake.notify_all();
    }

    pub fn set_playing(&self, playing: bool) {
        let mut state = self.state.lock().unwrap();
        let now = Instant::now();
        state.base_time = current_time_at(&state, now);
        state.anchor = now;
        state.playing = playing;
        state.generation += 1;
        self.wake.notify_all();
    }

    pub fn kill(&self) {
        let mut state = self.state.lock().unwrap();
        state.killed = true;
        state.generation += 1;
        self.wake.notify_all();
    }

    pub fn wait_until(&self, target_time: f64) -> WaitResult {
        let mut state = self.state.lock().unwrap();
        let generation = state.generation;

        loop {
            if state.killed {
                return WaitResult::Killed;
            }
            if state.generation != generation {
                return WaitResult::StateChanged;
            }
            if !state.playing {
                state = self.wake.wait(state).unwrap();
                continue;
            }

            let now = Instant::now();
            let current = current_time_at(&state, now);
            let remaining = target_time - current;
            if remaining <= 0.0 {
                return WaitResult::Reached;
            }

            let timeout = std::time::Duration::from_secs_f64(remaining.min(0.1));
            let (next, _) = self.wake.wait_timeout(state, timeout).unwrap();
            state = next;
        }
    }
}

fn current_time(state: &ClockState) -> f64 {
    current_time_at(state, Instant::now())
}

fn current_time_at(state: &ClockState, now: Instant) -> f64 {
    if state.playing {
        state.base_time + now.duration_since(state.anchor).as_secs_f64()
    } else {
        state.base_time
    }
}
