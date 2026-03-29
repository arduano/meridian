use serde::{Deserialize, Serialize};

use super::{commands::CoreCommand, events::CoreEvent, protocol_version};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRequest {
    #[serde(default = "protocol_version")]
    pub protocol_version: u32,
    pub id: Option<u64>,
    pub command: CoreCommand,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonResponse {
    #[serde(default = "protocol_version")]
    pub protocol_version: u32,
    pub id: Option<u64>,
    pub events: Vec<CoreEvent>,
}
