use std::{collections::HashMap, sync::Arc};

use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use landscape::service::ipv6pd::IPV6PDService;
use landscape_common::{
    config::dhcp_v6_client::IPV6PDServiceConfig,
    observer::IfaceObserverAction,
    service::{service_manager::ServiceManager, DefaultWatchServiceStatus},
    store::storev2::StoreFileManager,
};
use serde_json::Value;
use tokio::sync::{broadcast, Mutex};

use crate::{error::LandscapeApiError, SimpleResult};

#[derive(Clone)]
struct LandscapeIfaceIPV6ClientServices {
    service: ServiceManager<IPV6PDService>,
    store: Arc<Mutex<StoreFileManager<IPV6PDServiceConfig>>>,
}

pub async fn get_iface_pdclient_paths(
    mut store: StoreFileManager<IPV6PDServiceConfig>,
    mut dev_observer: broadcast::Receiver<IfaceObserverAction>,
) -> Router {
    let share_state = LandscapeIfaceIPV6ClientServices {
        service: ServiceManager::init(store.list()).await,
        store: Arc::new(Mutex::new(store)),
    };

    let share_state_copy = share_state.clone();
    tokio::spawn(async move {
        while let Ok(msg) = dev_observer.recv().await {
            match msg {
                IfaceObserverAction::Up(iface_name) => {
                    tracing::info!("restart {iface_name} NAT service");
                    let mut read_lock = share_state_copy.store.lock().await;
                    let service_config = if let Some(service_config) = read_lock.get(&iface_name) {
                        service_config
                    } else {
                        continue;
                    };
                    drop(read_lock);
                    let _ = share_state_copy.service.update_service(service_config).await;
                }
                IfaceObserverAction::Down(_) => {}
            }
        }
    });
    Router::new()
        .route("/ipv6pd/status", get(get_all_status))
        .route("/ipv6pd", post(handle_iface_pd))
        .route("/ipv6pd/:iface_name", get(get_iface_pd_conifg).delete(delete_and_stop_iface_pd))
        // .route("/nats/:iface_name/restart", post(restart_nat_service_status))
        .with_state(share_state)
}

async fn get_all_status(State(state): State<LandscapeIfaceIPV6ClientServices>) -> Json<Value> {
    let read_lock = state.service.services.read().await;
    let mut result = HashMap::new();
    for (key, (iface_status, _)) in read_lock.iter() {
        result.insert(key.clone(), iface_status.clone());
    }
    drop(read_lock);
    let result = serde_json::to_value(result);
    Json(result.unwrap())
}

async fn get_iface_pd_conifg(
    State(state): State<LandscapeIfaceIPV6ClientServices>,
    Path(iface_name): Path<String>,
) -> Result<Json<IPV6PDServiceConfig>, LandscapeApiError> {
    let mut read_lock = state.store.lock().await;
    if let Some(iface_config) = read_lock.get(&iface_name) {
        Ok(Json(iface_config))
    } else {
        Err(LandscapeApiError::NotFound("can not find".into()))
    }
}

/// 处理新建 IPv6 PD 获取配置
async fn handle_iface_pd(
    State(state): State<LandscapeIfaceIPV6ClientServices>,
    Json(service_config): Json<IPV6PDServiceConfig>,
) -> Json<Value> {
    let result = SimpleResult { success: true };

    if let Ok(()) = state.service.update_service(service_config.clone()).await {
        let mut write_lock = state.store.lock().await;
        write_lock.set(service_config);
        drop(write_lock);
    }
    let result = serde_json::to_value(result);
    Json(result.unwrap())
}

async fn delete_and_stop_iface_pd(
    State(state): State<LandscapeIfaceIPV6ClientServices>,
    Path(iface_name): Path<String>,
) -> Json<Value> {
    let mut write_lock = state.store.lock().await;
    write_lock.del(&iface_name);
    drop(write_lock);

    let mut write_lock = state.service.services.write().await;
    let data = if let Some((iface_status, _)) = write_lock.remove(&iface_name) {
        iface_status
    } else {
        DefaultWatchServiceStatus::new()
    };
    drop(write_lock);
    // 停止服务
    data.wait_stop().await;
    let result = serde_json::to_value(data);
    Json(result.unwrap())
}
