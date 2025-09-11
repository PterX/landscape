use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export, export_to = "common/docker.d.ts")]
pub struct PullImageReq {
    pub image_name: String,
    pub tag: Option<String>,
}

#[derive(Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "common/docker.d.ts")]
pub struct PullImgTask {
    pub id: Uuid,
    pub img_name: String,
    pub complete: bool,
    pub layer_current_info: HashMap<String, PullImgTaskItem>,
}

#[derive(Default, Clone, Serialize, Deserialize, Debug, TS)]
#[ts(export, export_to = "common/docker.d.ts")]
pub struct PullImgTaskItem {
    pub id: String,
    #[ts(type = "number | null")]
    pub current: Option<i64>,
    #[ts(type = "number | null")]
    pub total: Option<i64>,
}

#[derive(Clone, Serialize, Debug, TS)]
#[ts(export, export_to = "common/docker.d.ts")]
pub struct ImgPullEvent {
    pub task_id: Uuid,
    pub img_name: String,
    pub id: String,
    #[ts(type = "number | null")]
    pub current: Option<i64>,
    #[ts(type = "number | null")]
    pub total: Option<i64>,
}
