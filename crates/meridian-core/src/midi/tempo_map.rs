#[derive(Debug, Clone)]
pub struct TempoSegment {
    pub start_tick: u64,
    pub start_seconds: f64,
    pub micros_per_quarter: u32,
}

#[derive(Debug, Clone)]
pub struct TempoMap {
    ppq: u16,
    segments: Vec<TempoSegment>,
}

impl TempoMap {
    pub fn new(ppq: u16) -> Self {
        Self {
            ppq,
            segments: vec![TempoSegment {
                start_tick: 0,
                start_seconds: 0.0,
                micros_per_quarter: 500_000,
            }],
        }
    }

    pub fn ppq(&self) -> u16 {
        self.ppq
    }

    pub fn push_tempo_change(&mut self, tick: u64, seconds: f64, micros_per_quarter: u32) {
        if let Some(last) = self.segments.last_mut() {
            if last.start_tick == tick {
                last.start_seconds = seconds;
                last.micros_per_quarter = micros_per_quarter;
                return;
            }
            if last.micros_per_quarter == micros_per_quarter {
                return;
            }
        }
        self.segments.push(TempoSegment {
            start_tick: tick,
            start_seconds: seconds,
            micros_per_quarter,
        });
    }

    pub fn seconds_at_tick(&self, tick: u64) -> f64 {
        let segment = self.segment_for_tick(tick);
        segment.start_seconds
            + (tick.saturating_sub(segment.start_tick) as f64)
                * self.seconds_per_tick(segment.micros_per_quarter)
    }

    pub fn tick_at_seconds(&self, seconds: f64) -> f64 {
        let mut current = &self.segments[0];
        for next in self.segments.iter().skip(1) {
            if next.start_seconds > seconds {
                break;
            }
            current = next;
        }
        current.start_tick as f64
            + (seconds - current.start_seconds).max(0.0)
                / self.seconds_per_tick(current.micros_per_quarter)
    }

    pub fn ticks_per_second_at_seconds(&self, seconds: f64) -> f64 {
        let mut current = &self.segments[0];
        for next in self.segments.iter().skip(1) {
            if next.start_seconds > seconds {
                break;
            }
            current = next;
        }
        self.ticks_per_second(current.micros_per_quarter)
    }

    pub fn ticks_per_second_at_tick(&self, tick: u64) -> f64 {
        self.ticks_per_second(self.segment_for_tick(tick).micros_per_quarter)
    }

    fn segment_for_tick(&self, tick: u64) -> &TempoSegment {
        let index = self
            .segments
            .partition_point(|segment| segment.start_tick <= tick)
            .saturating_sub(1);
        &self.segments[index]
    }

    fn seconds_per_tick(&self, micros_per_quarter: u32) -> f64 {
        micros_per_quarter as f64 / 1_000_000.0 / self.ppq.max(1) as f64
    }

    fn ticks_per_second(&self, micros_per_quarter: u32) -> f64 {
        1.0 / self.seconds_per_tick(micros_per_quarter).max(f64::EPSILON)
    }
}

#[cfg(test)]
mod tests {
    use super::TempoMap;

    fn assert_approx_eq(actual: f64, expected: f64, epsilon: f64) {
        assert!(
            (actual - expected).abs() <= epsilon,
            "expected {expected}, got {actual} (|delta| = {})",
            (actual - expected).abs()
        );
    }

    #[test]
    fn desire_drive_tempo_staircase_round_trips_near_284_seconds() {
        // Extracted from `/mnt/fat/Midis/MIDIs/Impossible Piano - TH13 - Desire Drive.mid`.
        // This is the densest tempo-change cluster in the song and a good proxy for
        // tempo-sensitive tick-space rendering.
        let mut map = TempoMap::new(96);
        let segments = [
            (0_u64, 0.0_f64, 379_747_u32),
            (13_438, 53.156_669, 400_000),
            (14_210, 56.373_335, 387_097),
            (17_284, 68.768_504, 379_747),
            (28_802, 114.330_232, 400_000),
            (29_568, 117.521_899, 392_157),
            (32_640, 130.070_923, 384_615),
            (35_716, 142.394_629, 379_747),
            (50_304, 200.100_35, 382_166),
            (51_074, 203.165_64, 387_097),
            (54_148, 215.560_808, 379_747),
            (56_448, 224.658_913, 394_737),
            (57_216, 227.816_809, 379_747),
            (65_676, 261.282_014, 382_166),
            (66_432, 264.291_571, 392_157),
            (69_504, 276.840_595, 1_276_596),
            (69_888, 281.946_979, 714_286),
            (69_912, 282.125_551, 631_579),
            (69_936, 282.283_445, 612_245),
            (69_960, 282.436_507, 571_429),
            (70_008, 282.722_221, 606_061),
            (70_032, 282.873_736, 645_161),
            (70_056, 283.035_027, 705_882),
            (70_080, 283.211_497, 759_494),
            (70_104, 283.401_371, 779_221),
            (70_128, 283.596_176, 833_333),
            (70_152, 283.804_509, 857_143),
            (70_176, 284.018_795, 937_500),
            (70_200, 284.253_17, 1_034_483),
            (70_224, 284.511_791, 1_071_429),
            (70_248, 284.779_648, 1_714_286),
            (70_272, 285.208_219, 500_000),
        ];

        for (tick, seconds, mpq) in segments {
            map.push_tempo_change(tick, seconds, mpq);
        }

        // Anchor the region around the sharp slowdown at 284.25s. This is the most
        // useful mark to test in the UI because the local ticks/second change several
        // times within roughly one second.
        assert_approx_eq(map.tick_at_seconds(284.25), 70_199.675_392, 0.01);
        assert_approx_eq(map.ticks_per_second_at_seconds(284.25), 102.4, 1e-9);

        // Verify exact boundaries across the staircase so tick<->time conversion
        // remains stable at each tempo transition.
        assert_approx_eq(map.seconds_at_tick(70_176), 284.018_795, 1e-6);
        assert_approx_eq(map.seconds_at_tick(70_200), 284.253_17, 1e-6);
        assert_approx_eq(map.seconds_at_tick(70_224), 284.511_791, 1e-6);
        assert_approx_eq(map.seconds_at_tick(70_248), 284.779_648, 1e-6);
        assert_approx_eq(map.seconds_at_tick(70_272), 285.208_219, 1e-6);

        assert_approx_eq(map.tick_at_seconds(284.018_795), 70_176.0, 1e-6);
        assert_approx_eq(map.tick_at_seconds(284.253_17), 70_200.0, 1e-6);
        assert_approx_eq(map.tick_at_seconds(284.511_791), 70_224.0, 1e-6);
        assert_approx_eq(map.tick_at_seconds(284.779_648), 70_248.0, 1e-6);
        assert_approx_eq(map.tick_at_seconds(285.208_219), 70_272.0, 1e-6);
    }
}
