#[repr(C)]
#[derive(
    Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable, serde::Serialize, serde::Deserialize,
)]
pub struct NoteInstance {
    pub key: u32,
    pub start: f32,
    pub end: f32,
    pub left_color: u32,
    pub right_color: u32,
}

impl NoteInstance {
    pub fn new(key: u32, start: f32, end: f32, left_color: u32, right_color: u32) -> Self {
        Self {
            key,
            start,
            end,
            left_color,
            right_color,
        }
    }
}
