use midi_toolkit::{
    events::Event,
    sequence::event::{Delta, EventBatch, Track},
};

use crate::error::MeridianError;

pub(crate) trait TraversalEvent {
    fn track(&self) -> u32;
    fn as_event(&self) -> &Event;
}

impl<D> TraversalEvent for Delta<D, Track<Event>>
where
    D: midi_toolkit::num::MIDINum,
{
    fn track(&self) -> u32 {
        self.event.track
    }

    fn as_event(&self) -> &Event {
        &self.event.event
    }
}

impl<D> TraversalEvent for Delta<D, Track<&Event>>
where
    D: midi_toolkit::num::MIDINum,
{
    fn track(&self) -> u32 {
        self.event.track
    }

    fn as_event(&self) -> &Event {
        self.event.event
    }
}

pub(crate) trait MergedMidiTraversalItem {
    fn visit_events<E>(
        &self,
        visit: impl FnMut(&dyn TraversalEvent) -> Result<(), E>,
    ) -> Result<(), E>;
}

impl<D> MergedMidiTraversalItem for Delta<D, Track<Event>>
where
    D: midi_toolkit::num::MIDINum,
{
    fn visit_events<E>(
        &self,
        mut visit: impl FnMut(&dyn TraversalEvent) -> Result<(), E>,
    ) -> Result<(), E> {
        visit(self)
    }
}

impl<D> MergedMidiTraversalItem for Delta<D, Track<EventBatch<Event>>>
where
    D: midi_toolkit::num::MIDINum,
{
    fn visit_events<E>(
        &self,
        mut visit: impl FnMut(&dyn TraversalEvent) -> Result<(), E>,
    ) -> Result<(), E> {
        self.iter_events().try_for_each(|event| visit(&event))
    }
}

pub(crate) fn walk_merged_midi_items<I, Item, State, OnItem, OnEvent, OnItemEnd>(
    items: I,
    mut should_cancel: impl FnMut() -> bool,
    state: &mut State,
    mut on_item: OnItem,
    mut on_event: OnEvent,
    mut on_item_end: OnItemEnd,
) -> Result<(), MeridianError>
where
    I: IntoIterator<Item = Item>,
    Item: MergedMidiTraversalItem,
    OnItem: FnMut(&mut State, &Item) -> Result<(), MeridianError>,
    OnEvent: FnMut(&mut State, &dyn TraversalEvent) -> Result<(), MeridianError>,
    OnItemEnd: FnMut(&mut State, &Item) -> Result<(), MeridianError>,
{
    for item in items {
        if should_cancel() {
            return Err(MeridianError::Cancelled("midi load cancelled".into()));
        }
        on_item(state, &item)?;
        item.visit_events(|event| on_event(state, event))?;
        on_item_end(state, &item)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use midi_toolkit::sequence::event::{Delta, Track};

    use super::{MergedMidiTraversalItem, TraversalEvent, walk_merged_midi_items};
    use crate::MeridianError;
    use crate::midi::test_support::{note_off, note_on};

    struct TestItem {
        delta: u64,
        track: u32,
        events: Vec<Delta<u64, Track<midi_toolkit::events::Event>>>,
    }

    impl MergedMidiTraversalItem for TestItem {
        fn visit_events<E>(
            &self,
            mut visit: impl FnMut(&dyn TraversalEvent) -> Result<(), E>,
        ) -> Result<(), E> {
            for event in &self.events {
                visit(event)?;
            }
            Ok(())
        }
    }

    #[test]
    fn walks_items_and_events_in_order() {
        let items = vec![
            TestItem {
                delta: 3,
                track: 2,
                events: vec![
                    Delta::new(3, Track::new(note_on(0, 1, 60, 100).event, 2)),
                    Delta::new(0, Track::new(note_off(0, 1, 60).event, 2)),
                ],
            },
            TestItem {
                delta: 7,
                track: 4,
                events: vec![Delta::new(7, Track::new(note_on(0, 3, 65, 90).event, 4))],
            },
        ];

        struct TestState {
            seen: Vec<String>,
        }

        let mut state = TestState { seen: Vec::new() };
        walk_merged_midi_items(
            items,
            || false,
            &mut state,
            |state, item| {
                state.seen.push(format!("item:{}:{}", item.track, item.delta));
                Ok(())
            },
            |state, event| {
                state
                    .seen
                    .push(format!("event:{}:{:?}", event.track(), event.as_event()));
                Ok(())
            },
            |_, _| Ok(()),
        )
        .expect("walk should succeed");

        assert_eq!(
            state.seen,
            vec![
                "item:2:3",
                "event:2:NoteOn(NoteOnEvent { channel: 1, key: 60, velocity: 100 })",
                "event:2:NoteOff(NoteOffEvent { channel: 1, key: 60 })",
                "item:4:7",
                "event:4:NoteOn(NoteOnEvent { channel: 3, key: 65, velocity: 90 })",
            ]
        );
    }

    #[test]
    fn stops_before_next_item_when_cancelled() {
        let items = vec![
            TestItem {
                delta: 1,
                track: 1,
                events: vec![Delta::new(1, Track::new(note_on(0, 0, 60, 1).event, 1))],
            },
            TestItem {
                delta: 2,
                track: 2,
                events: vec![Delta::new(2, Track::new(note_on(0, 0, 61, 1).event, 2))],
            },
        ];

        struct TestState {
            seen: Vec<u32>,
        }

        let mut state = TestState { seen: Vec::new() };
        let mut item_count = 0;
        let err = walk_merged_midi_items(
            items,
            || {
                item_count += 1;
                item_count > 1
            },
            &mut state,
            |state, item| {
                state.seen.push(item.track);
                Ok(())
            },
            |_, _| Ok(()),
            |_, _| Ok(()),
        )
        .expect_err("walk should cancel");

        assert!(matches!(err, MeridianError::Cancelled(_)));
        assert_eq!(state.seen, vec![1]);
    }
}
