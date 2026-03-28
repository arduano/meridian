use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct MiditrailVertex {
    pub position: [f32; 3],
    pub color: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct MiditrailAuraVertex {
    pub position: [f32; 3],
    pub color: [f32; 4],
    pub uv: [f32; 2],
    pub _padding: [f32; 2],
}

#[derive(Clone, Debug, Default)]
pub struct MiditrailScene {
    pub note_vertices: Vec<MiditrailVertex>,
    pub white_key_vertices: Vec<MiditrailVertex>,
    pub black_key_vertices: Vec<MiditrailVertex>,
    pub aura_vertices: Vec<MiditrailAuraVertex>,
}

impl MiditrailScene {
    pub fn clear(&mut self) {
        self.note_vertices.clear();
        self.white_key_vertices.clear();
        self.black_key_vertices.clear();
        self.aura_vertices.clear();
    }

    pub fn total_vertices(&self) -> usize {
        self.note_vertices.len()
            + self.white_key_vertices.len()
            + self.black_key_vertices.len()
            + self.aura_vertices.len()
    }
}
