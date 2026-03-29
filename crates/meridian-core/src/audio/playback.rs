use std::{sync::Arc, thread::{self, JoinHandle}};

use crate::midi::audio_cache::{CompressedAudio, InRamAudioCache};

use super::{PlaybackClock, clock::WaitResult, player::MeridianAudioPlayer};

pub struct LiveAudioSession {
    clock: Arc<PlaybackClock>,
    thread: Option<JoinHandle<()>>,
}

impl LiveAudioSession {
    pub fn spawn(
        events: Arc<InRamAudioCache>,
        clock: Arc<PlaybackClock>,
        player: Arc<MeridianAudioPlayer>,
    ) -> Self {
        let clock_for_thread = Arc::clone(&clock);
        let thread = thread::spawn(move || {
            let mut index = 0usize;
            loop {
                if index >= events.events().len() {
                    match clock_for_thread.wait_until(f64::INFINITY) {
                        WaitResult::Killed => break,
                        WaitResult::StateChanged => {
                            index = seek_to_time(events.events(), clock_for_thread.snapshot_time(), &player);
                        }
                        WaitResult::Reached => unreachable!(),
                    }
                    continue;
                }

                let event = &events.events()[index];
                match clock_for_thread.wait_until(event.time) {
                    WaitResult::Reached => {
                        player.push_events(event.iter_events());
                        index += 1;
                    }
                    WaitResult::StateChanged => {
                        index = seek_to_time(events.events(), clock_for_thread.snapshot_time(), &player);
                    }
                    WaitResult::Killed => {
                        player.reset();
                        break;
                    }
                }
            }
        });

        Self {
            clock,
            thread: Some(thread),
        }
    }
}

impl Drop for LiveAudioSession {
    fn drop(&mut self) {
        self.clock.kill();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn seek_to_time(events: &[CompressedAudio], time: f64, player: &MeridianAudioPlayer) -> usize {
    let index = find_time_index(events, time);
    player.reset();
    for event in &events[..index] {
        player.push_events(event.iter_control_events());
    }
    index
}

fn find_time_index(events: &[CompressedAudio], time: f64) -> usize {
    if time < 0.0 {
        return 0;
    }

    let mut size = events.len();
    let mut left = 0;
    let mut right = size;
    while left < right {
        let mid = left + size / 2;
        let range_start = events.get(mid.saturating_sub(1)).map(|t| t.time).unwrap_or(f64::NEG_INFINITY);
        let range_end = events[mid].time;
        if time < range_start {
            right = mid;
        } else if time > range_end {
            left = mid + 1;
        } else {
            return mid;
        }
        size = right - left;
    }
    events.len()
}
