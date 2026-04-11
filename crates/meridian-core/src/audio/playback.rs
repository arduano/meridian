use std::{
    sync::Arc,
    sync::mpsc,
    thread::{self, JoinHandle},
};

use crate::midi::audio_cache::{CompressedAudio, InRamAudioCache};

use super::{PlaybackClock, clock::WaitResult, player::MeridianAudioPlayer};

pub struct LiveAudioSession {
    clock: Arc<PlaybackClock>,
    thread: Option<JoinHandle<()>>,
}

trait SessionPlayer: Send + Sync + 'static {
    fn push_events<I>(&self, data: I)
    where
        I: Iterator<Item = u32>;

    fn reset(&self);
}

impl SessionPlayer for MeridianAudioPlayer {
    fn push_events<I>(&self, data: I)
    where
        I: Iterator<Item = u32>,
    {
        MeridianAudioPlayer::push_events(self, data);
    }

    fn reset(&self) {
        MeridianAudioPlayer::reset(self);
    }
}

impl LiveAudioSession {
    pub fn spawn(
        events: Arc<InRamAudioCache>,
        clock: Arc<PlaybackClock>,
        player: Arc<MeridianAudioPlayer>,
    ) -> Self {
        let (ready_tx, ready_rx) = mpsc::channel();
        let thread = spawn_session_thread(events, Arc::clone(&clock), player, move || {
            let _ = ready_tx.send(());
        });
        let _ = ready_rx.recv();

        Self {
            clock,
            thread: Some(thread),
        }
    }
}

fn spawn_session_thread<P>(
    events: Arc<InRamAudioCache>,
    clock: Arc<PlaybackClock>,
    player: Arc<P>,
    on_ready: impl FnOnce() + Send + 'static,
) -> JoinHandle<()>
where
    P: SessionPlayer,
{
    thread::spawn(move || {
        let mut on_ready = Some(on_ready);
        let mut index = seek_to_time(events.events(), clock.snapshot_base_time(), player.as_ref());
        on_ready
            .take()
            .expect("audio session ready callback already used")();

        loop {
            if index >= events.events().len() {
                match clock.wait_until(f64::INFINITY) {
                    WaitResult::Killed => break,
                    WaitResult::StateChanged => {
                        index = seek_to_time(
                            events.events(),
                            clock.snapshot_base_time(),
                            player.as_ref(),
                        );
                    }
                    WaitResult::Reached => unreachable!(),
                }
                continue;
            }

            let event = &events.events()[index];
            match clock.wait_until(event.time) {
                WaitResult::Reached => {
                    player.push_events(event.iter_events());
                    index += 1;
                }
                WaitResult::StateChanged => {
                    index =
                        seek_to_time(events.events(), clock.snapshot_base_time(), player.as_ref());
                }
                WaitResult::Killed => {
                    player.reset();
                    break;
                }
            }
        }
    })
}

impl Drop for LiveAudioSession {
    fn drop(&mut self) {
        self.clock.kill();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn seek_to_time<P>(events: &[CompressedAudio], time: f64, player: &P) -> usize
where
    P: SessionPlayer + ?Sized,
{
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

    events.partition_point(|event| event.time < time)
}

#[cfg(test)]
mod tests {
    use std::{
        sync::{
            Mutex,
            atomic::{AtomicUsize, Ordering},
        },
        time::Duration,
    };

    use super::*;

    #[derive(Default)]
    struct TestPlayer {
        events: Mutex<Vec<u32>>,
        resets: AtomicUsize,
    }

    impl TestPlayer {
        fn events(&self) -> Vec<u32> {
            self.events
                .lock()
                .expect("test player mutex poisoned")
                .clone()
        }

        fn reset_count(&self) -> usize {
            self.resets.load(Ordering::SeqCst)
        }
    }

    impl SessionPlayer for TestPlayer {
        fn push_events<I>(&self, data: I)
        where
            I: Iterator<Item = u32>,
        {
            self.events
                .lock()
                .expect("test player mutex poisoned")
                .extend(data);
        }

        fn reset(&self) {
            self.resets.fetch_add(1, Ordering::SeqCst);
        }
    }

    #[test]
    fn session_waits_for_play_before_emitting_start_events() {
        let events = Arc::new(InRamAudioCache::new(vec![
            CompressedAudio::from_parts(0.0, vec![0x90, 60, 100], Some(vec![])),
            CompressedAudio::from_parts(0.01, vec![0x80, 60], Some(vec![])),
        ]));
        let clock = Arc::new(PlaybackClock::new());
        let player = Arc::new(TestPlayer::default());
        let (ready_tx, ready_rx) = mpsc::channel();
        let handle =
            spawn_session_thread(events, Arc::clone(&clock), Arc::clone(&player), move || {
                let _ = ready_tx.send(());
            });

        ready_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("session worker should report ready");
        assert!(player.events().is_empty());

        clock.set_playing(true);
        thread::sleep(Duration::from_millis(40));
        clock.kill();
        handle.join().expect("session thread should join");

        assert_eq!(
            player.events(),
            vec![(0x90_u32) | (60 << 8) | (100 << 16), (0x80_u32) | (60 << 8)]
        );
        assert!(player.reset_count() >= 1);
    }

    #[test]
    fn session_starts_from_the_current_clock_position() {
        let events = Arc::new(InRamAudioCache::new(vec![
            CompressedAudio::from_parts(0.0, vec![0x90, 60, 100], Some(vec![])),
            CompressedAudio::from_parts(0.01, vec![0x80, 60], Some(vec![])),
        ]));
        let clock = Arc::new(PlaybackClock::new());
        clock.set_time(0.02);
        let player = Arc::new(TestPlayer::default());
        let (ready_tx, ready_rx) = mpsc::channel();
        let handle =
            spawn_session_thread(events, Arc::clone(&clock), Arc::clone(&player), move || {
                let _ = ready_tx.send(());
            });

        ready_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("session worker should report ready");
        thread::sleep(Duration::from_millis(20));
        clock.kill();
        handle.join().expect("session thread should join");

        assert!(player.events().is_empty());
        assert!(player.reset_count() >= 1);
    }

    #[test]
    fn seek_index_keeps_events_at_the_exact_target_time() {
        let events = vec![
            CompressedAudio::from_parts(0.0, vec![0x90, 60, 100], Some(vec![])),
            CompressedAudio::from_parts(0.01, vec![0x80, 60], Some(vec![])),
        ];

        assert_eq!(find_time_index(&events, 0.0), 0);
        assert_eq!(find_time_index(&events, 0.005), 1);
        assert_eq!(find_time_index(&events, 0.01), 1);
        assert_eq!(find_time_index(&events, 0.02), 2);
    }
}
