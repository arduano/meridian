use enum_dispatch::enum_dispatch;

use crate::render::DisplayTimeSpace;

use super::{DisplacedMIDINote, MIDIFileStats, MIDIFileUniqueSignature, MIDIViewRange};

#[enum_dispatch]
pub trait MIDIFileBase {
    fn midi_length(&self) -> Option<f64>;
    fn parsed_up_to(&self) -> Option<f64>;
    fn stats(&self) -> MIDIFileStats;
    fn allows_seeking_backward(&self) -> bool;
    fn signature(&self) -> &MIDIFileUniqueSignature;
}

pub trait MIDIFile: MIDIFileBase {
    type ColumnsViews<'a>: 'a + MIDINoteViews
    where
        Self: 'a;

    fn get_current_column_views(
        &mut self,
        time: f64,
        range: f64,
        time_space: DisplayTimeSpace,
    ) -> Self::ColumnsViews<'_>;
}

pub trait MIDINoteViews {
    type View<'a>: 'a + MIDINoteColumnView
    where
        Self: 'a;

    fn get_column(&self, key: usize) -> Self::View<'_>;
    fn range(&self) -> MIDIViewRange;
}

pub trait MIDINoteColumnView: Send {
    type Iter<'a>: 'a + ExactSizeIterator<Item = DisplacedMIDINote> + Send
    where
        Self: 'a;

    fn iterate_displaced_notes(&self) -> Self::Iter<'_>;
}
