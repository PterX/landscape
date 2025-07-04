use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use landscape_common::service::controller_service::{ConfigController, FlowConfigController};
use landscape_common::{
    config::{ConfigId, FlowId},
    ip_mark::WanIpRuleConfig,
};

use crate::{
    error::{LandscapeApiError, LandscapeResult},
    LandscapeApp, SimpleResult,
};

pub async fn get_dst_ip_rule_config_paths() -> Router<LandscapeApp> {
    Router::new()
        .route("/dst_ip_rules", get(get_dst_ip_rules).post(add_dst_ip_rules))
        .route("/dst_ip_rules/set_many", post(add_many_dst_ip_rules))
        .route(
            "/dst_ip_rules/:id",
            get(get_dst_ip_rule).post(modify_dst_ip_rules).delete(del_dst_ip_rule),
        )
        .route("/dst_ip_rules/flow/:flow_id", get(get_flow_dst_ip_rules))
}

async fn get_dst_ip_rules(State(state): State<LandscapeApp>) -> Json<Vec<WanIpRuleConfig>> {
    let result = state.dst_ip_rule_service.list().await;
    Json(result)
}

async fn get_flow_dst_ip_rules(
    State(state): State<LandscapeApp>,
    Path(id): Path<FlowId>,
) -> Json<Vec<WanIpRuleConfig>> {
    let mut result = state.dst_ip_rule_service.list_flow_configs(id).await;
    result.sort_by(|a, b| a.index.cmp(&b.index));
    Json(result)
}

async fn get_dst_ip_rule(
    State(state): State<LandscapeApp>,
    Path(id): Path<ConfigId>,
) -> LandscapeResult<Json<WanIpRuleConfig>> {
    let result = state.dst_ip_rule_service.find_by_id(id).await;
    if let Some(config) = result {
        Ok(Json(config))
    } else {
        Err(LandscapeApiError::NotFound(format!("DstIpRule id: {:?}", id)))
    }
}

async fn modify_dst_ip_rules(
    State(state): State<LandscapeApp>,
    Path(_id): Path<ConfigId>,
    Json(rule): Json<WanIpRuleConfig>,
) -> Json<WanIpRuleConfig> {
    let result = state.dst_ip_rule_service.set(rule).await;
    Json(result)
}

async fn add_dst_ip_rules(
    State(state): State<LandscapeApp>,
    Json(rule): Json<WanIpRuleConfig>,
) -> Json<WanIpRuleConfig> {
    let result = state.dst_ip_rule_service.set(rule).await;
    Json(result)
}

async fn add_many_dst_ip_rules(
    State(state): State<LandscapeApp>,
    Json(rules): Json<Vec<WanIpRuleConfig>>,
) -> Json<SimpleResult> {
    state.dst_ip_rule_service.set_list(rules).await;
    Json(SimpleResult { success: true })
}

async fn del_dst_ip_rule(
    State(state): State<LandscapeApp>,
    Path(id): Path<ConfigId>,
) -> Json<SimpleResult> {
    state.dst_ip_rule_service.delete(id).await;
    Json(SimpleResult { success: true })
}
