use axum::{
    extract::{Path, State},
    routing::get,
    Json, Router,
};
use landscape_common::service::controller_service::ConfigController;
use landscape_common::{config::ConfigId, flow::FlowConfig};

use crate::{error::LandscapeApiError, LandscapeApp};

use crate::{api::LandscapeApiResp, error::LandscapeApiResult};

pub async fn get_flow_rule_config_paths() -> Router<LandscapeApp> {
    Router::new()
        .route("/flow_rules", get(get_flow_rules).post(add_flow_rule))
        .route("/flow_rules/{id}", get(get_flow_rule).delete(del_flow_rule))
}

async fn get_flow_rules(State(state): State<LandscapeApp>) -> LandscapeApiResult<Vec<FlowConfig>> {
    let result = state.flow_rule_service.list().await;
    LandscapeApiResp::success(result)
}

async fn get_flow_rule(
    State(state): State<LandscapeApp>,
    Path(id): Path<ConfigId>,
) -> LandscapeApiResult<FlowConfig> {
    let result = state.flow_rule_service.find_by_id(id).await;
    if let Some(config) = result {
        LandscapeApiResp::success(config)
    } else {
        Err(LandscapeApiError::NotFound(format!("Flow rule id: {:?}", id)))
    }
}

async fn add_flow_rule(
    State(state): State<LandscapeApp>,
    Json(flow_rule): Json<FlowConfig>,
) -> LandscapeApiResult<FlowConfig> {
    let result = state.flow_rule_service.set(flow_rule).await;
    LandscapeApiResp::success(result)
}

async fn del_flow_rule(
    State(state): State<LandscapeApp>,
    Path(id): Path<ConfigId>,
) -> LandscapeApiResult<()> {
    state.flow_rule_service.delete(id).await;
    LandscapeApiResp::success(())
}
