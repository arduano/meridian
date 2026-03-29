use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct MiditrailQuadInstance {
    pub positions: [[f32; 3]; 4],
    pub colors: [[f32; 4]; 4],
}

#[derive(Clone, Debug, Default)]
pub struct MiditrailScene {
    pub note_quads: Vec<MiditrailQuadInstance>,
    pub white_key_quads: Vec<MiditrailQuadInstance>,
    pub black_key_quads: Vec<MiditrailQuadInstance>,
    pub aura_quads: Vec<MiditrailQuadInstance>,
}

impl MiditrailScene {
    pub fn clear(&mut self) {
        self.note_quads.clear();
        self.white_key_quads.clear();
        self.black_key_quads.clear();
        self.aura_quads.clear();
    }

    pub fn total_vertices(&self) -> usize {
        (self.note_quads.len()
            + self.white_key_quads.len()
            + self.black_key_quads.len()
            + self.aura_quads.len())
            * 6
    }
}
