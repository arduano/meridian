use super::*;

mod controls;
use controls::*;
mod float;
use float::*;
mod assets;
use assets::*;

pub(super) fn wire_video_callbacks(
    app: &App,
    bridge: &UiCoreBridge,
    shared_state: &Arc<Mutex<UiViewModel>>,
) {
    wire_video_control_callbacks(app, bridge, shared_state);
    wire_video_float_callbacks(app, bridge, shared_state);
    wire_video_asset_callbacks(app, bridge, shared_state);
}
