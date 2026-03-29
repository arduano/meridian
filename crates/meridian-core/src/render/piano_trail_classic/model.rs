use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct PianoTrailClassicQuadInstance {
    pub positions: [[f32; 3]; 4],
    pub colors: [[f32; 4]; 4],
}

#[derive(Clone, Debug, Default)]
pub struct PianoTrailClassicScene {
    pub note_quads: Vec<PianoTrailClassicQuadInstance>,
    pub white_key_quads: Vec<PianoTrailClassicQuadInstance>,
    pub black_key_quads: Vec<PianoTrailClassicQuadInstance>,
    pub aura_quads: Vec<PianoTrailClassicQuadInstance>,
}

impl PianoTrailClassicScene {
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
