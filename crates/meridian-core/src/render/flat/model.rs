use super::super::shared::KeyActivity;

#[derive(Clone, Copy)]
pub(crate) struct FlatKeyState {
    pub x1: f32,
    pub x2: f32,
    pub is_black: bool,
    pub activity: KeyActivity,
}
