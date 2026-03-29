mod config;
mod math;
mod note_colors;
mod physics;
mod projector_image;
mod scene;

pub(crate) const LAYER_COUNT: usize = 7;

pub use config::*;
pub(crate) use math::*;
pub use note_colors::*;
pub use physics::*;
pub use projector_image::*;
pub use scene::*;
