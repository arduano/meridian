use midi_toolkit::{
    events::{Event, MIDIEventEnum},
    pipe,
    sequence::{
        TimeCaster,
        event::{Delta, EventBatch, Track, cancel_tempo_events, scale_event_time},
        unwrap_items,
    },
};

use crate::{error::MeridianError, midi::parsed::ParsedMidiFile};

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
    pub fn from_parsed(parsed: &ParsedMidiFile) -> Result<Self, MeridianError> {
        let midi = parsed.midi();
        let ppq = midi.ppq();
        if (ppq & 0x8000) != 0 {
            return Err(MeridianError::InvalidMidi(
                "timecode MIDI files are not supported yet".into(),
            ));
        }

        let merged = pipe!(
            midi.iter_all_track_events_merged_batches()
            |>TimeCaster::<f64>::cast_event_delta()
            |>cancel_tempo_events(250000)
            |>scale_event_time(1.0 / ppq as f64)
            |>unwrap_items()
        );

        type Ev = Delta<f64, Track<EventBatch<Event>>>;
        let mut time = 0.0;
        let mut out = Vec::new();

        for block in merged {
            let block: Ev = block;
            time += block.delta;
            let mut data = Vec::with_capacity(block.count() * 3);
            let mut control = Vec::new();

            for event in block.iter_events() {
                match event.as_event() {
                    Event::NoteOn(e) => {
                        data.extend_from_slice(&[EV_ON | e.channel, e.key, e.velocity])
                    }
                    Event::NoteOff(e) => data.extend_from_slice(&[EV_OFF | e.channel, e.key]),
                    Event::PolyphonicKeyPressure(e) => {
                        data.extend_from_slice(&[EV_POLYPHONIC | e.channel, e.key, e.velocity])
                    }
                    Event::ControlChange(e) => {
                        let bytes = [EV_CONTROL | e.channel, e.controller, e.value];
                        data.extend_from_slice(&bytes);
                        control.extend_from_slice(&bytes);
                    }
                    Event::ProgramChange(e) => {
                        let bytes = [EV_PROGRAM | e.channel, e.program];
                        data.extend_from_slice(&bytes);
                        control.extend_from_slice(&bytes);
                    }
                    Event::ChannelPressure(e) => {
                        let bytes = [EV_CHAN_PRESSURE | e.channel, e.pressure];
                        data.extend_from_slice(&bytes);
                        control.extend_from_slice(&bytes);
                    }
                    Event::PitchWheelChange(e) => {
                        let value = e.pitch + 8192;
                        let bytes = [
                            EV_PITCH_BEND | e.channel,
                            (value & 0x7F) as u8,
                            ((value >> 7) & 0x7F) as u8,
                        ];
                        data.extend_from_slice(&bytes);
                        control.extend_from_slice(&bytes);
                    }
                    _ => {}
                }
            }

            out.push(CompressedAudio {
                time,
                data,
                control_only_data: (!control.is_empty()).then_some(control),
            });
        }

        Ok(Self { events: out })
    }

    pub fn events(&self) -> &[CompressedAudio] {
        &self.events
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
