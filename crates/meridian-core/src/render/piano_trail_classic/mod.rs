pub mod debug;
pub mod layout;
pub mod model;
pub mod physics;
pub mod projector;
pub mod wgpu;

pub use model::PianoTrailClassicScene;
pub use physics::{PianoTrailClassicPhysicsState, tick_piano_trail_classic_physics};
pub use projector::project_piano_trail_classic_scene;
