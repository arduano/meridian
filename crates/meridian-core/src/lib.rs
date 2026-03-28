pub mod engine;
pub mod error;
pub mod midi;
pub mod protocol;
pub mod render;

pub use engine::{CoreHandle, CoreResponse, spawn_core};
pub use error::MeridianError;
pub use protocol::{PROTOCOL_VERSION, RenderedFrame};
