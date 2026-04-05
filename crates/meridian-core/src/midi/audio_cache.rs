use crate::{
    error::MeridianError,
    midi::{
        materialized::{MaterializeOptions, build_materialized_midi_with_progress},
        parsed::ParsedMidiFile,
    },
};

#[derive(Clone, Debug)]
pub struct CompressedAudio {
    pub time: f64,
    data: Vec<u8>,
    control_only_data: Option<Vec<u8>>,
}

pub struct InRamAudioCache {
    events: Vec<CompressedAudio>,
}

const EV_OFF: u8 = 0x80;
const EV_ON: u8 = 0x90;
const EV_POLYPHONIC: u8 = 0xA0;
const EV_CONTROL: u8 = 0xB0;
const EV_PROGRAM: u8 = 0xC0;
const EV_CHAN_PRESSURE: u8 = 0xD0;
const EV_PITCH_BEND: u8 = 0xE0;

impl InRamAudioCache {
    pub(crate) fn new(events: Vec<CompressedAudio>) -> Self {
        Self { events }
    }

    pub fn from_parsed(parsed: &ParsedMidiFile) -> Result<Self, MeridianError> {
        Self::from_parsed_with_progress(parsed, |_| {})
    }

    pub fn from_parsed_with_progress(
        parsed: &ParsedMidiFile,
        progress: impl FnMut(crate::midi::MidiBuildProgress),
    ) -> Result<Self, MeridianError> {
        let materialized = build_materialized_midi_with_progress(
            parsed,
            MaterializeOptions {
                display: false,
                audio: true,
            },
            progress,
        )?;
        Ok(materialized
            .audio
            .expect("audio cache must exist when audio materialization is requested"))
    }

    pub fn events(&self) -> &[CompressedAudio] {
        &self.events
    }

    pub fn length(&self) -> f64 {
        self.events.last().map(|event| event.time).unwrap_or(0.0)
    }
}

impl CompressedAudio {
    pub(crate) fn from_parts(time: f64, data: Vec<u8>, control_only_data: Option<Vec<u8>>) -> Self {
        Self {
            time,
            data,
            control_only_data,
        }
    }
}

impl CompressedAudio {
    pub fn iter_events(&self) -> impl '_ + Iterator<Item = u32> {
        Self::iter_events_from_vec(self.data.iter().copied())
    }

    pub fn iter_control_events(&self) -> impl '_ + Iterator<Item = u32> {
        Self::iter_events_from_vec(self.control_only_data.iter().flatten().copied())
    }

    fn iter_events_from_vec<'a>(
        mut iter: impl 'a + Iterator<Item = u8>,
    ) -> impl 'a + Iterator<Item = u32> {
        std::iter::from_fn(move || {
            let next = iter.next()?;
            let ev = next & 0xF0;
            Some(match ev {
                EV_OFF | EV_PROGRAM | EV_CHAN_PRESSURE => {
                    let val2 = iter.next().unwrap() as u32;
                    (next as u32) | (val2 << 8)
                }
                EV_ON | EV_POLYPHONIC | EV_CONTROL | EV_PITCH_BEND => {
                    let val2 = iter.next().unwrap() as u32;
                    let val3 = iter.next().unwrap() as u32;
                    (next as u32) | (val2 << 8) | (val3 << 16)
                }
                _ => unreachable!("invalid packed MIDI event {next:#x}"),
            })
        })
    }
}
