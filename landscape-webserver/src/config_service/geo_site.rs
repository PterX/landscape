use axum::{
    extract::{DefaultBodyLimit, Multipart, Path, Query, State},
    routing::{get, post},
    Json, Router,
};
use landscape_common::config::{
    geo::{
        GeoDomainConfig, GeoFileCacheKey, GeoSiteSourceConfig, QueryGeoDomainConfig, QueryGeoKey,
    },
    ConfigId,
};
use landscape_common::service::controller_service_v2::ConfigController;

use crate::{api::LandscapeApiResp, error::LandscapeApiResult, UPLOAD_GEO_FILE_SIZE_LIMIT};
use crate::{error::LandscapeApiError, LandscapeApp};

pub async fn get_geo_site_config_paths() -> Router<LandscapeApp> {
    Router::new()
        .route("/geo_sites", get(get_geo_sites).post(add_geo_site))
        .route("/geo_sites/set_many", post(add_many_geo_sites))
        .route("/geo_sites/{id}", get(get_geo_rule).delete(del_geo_site))
        .route("/geo_sites/cache", get(get_geo_site_cache).post(refresh_geo_site_cache))
        .route("/geo_sites/cache/search", get(search_geo_site_cache))
        .route("/geo_sites/cache/detail", get(get_geo_site_cache_detail))
        .route(
            "/geo_sites/{name}/update_by_upload",
            post(update_by_upload).layer(DefaultBodyLimit::max(UPLOAD_GEO_FILE_SIZE_LIMIT)),
        )
}

async fn get_geo_site_cache_detail(
    State(state): State<LandscapeApp>,
    Query(key): Query<GeoFileCacheKey>,
) -> LandscapeApiResult<GeoDomainConfig> {
    let result = state.geo_site_service.get_cache_value_by_key(&key).await;
    if let Some(result) = result {
        LandscapeApiResp::success(result)
    } else {
        Err(LandscapeApiError::NotFound(format!("{key:?}")))
    }
}

async fn search_geo_site_cache(
    State(state): State<LandscapeApp>,
    Query(query): Query<QueryGeoKey>,
) -> LandscapeApiResult<Vec<GeoFileCacheKey>> {
    tracing::debug!("query: {:?}", query);
    let key = query.key.map(|k| k.to_ascii_uppercase());
    let name = query.name;
    tracing::debug!("name: {name:?}");
    tracing::debug!("key: {key:?}");
    let result: Vec<GeoFileCacheKey> = state
        .geo_site_service
        .list_all_keys()
        .await
        .into_iter()
        .filter(|e| key.as_ref().map_or(true, |key| e.key.contains(key)))
        .filter(|e| name.as_ref().map_or(true, |name| e.name.contains(name)))
        .collect();

    tracing::debug!("keys len: {}", result.len());
    LandscapeApiResp::success(result)
}

async fn get_geo_site_cache(
    State(state): State<LandscapeApp>,
) -> LandscapeApiResult<Vec<GeoFileCacheKey>> {
    let result = state.geo_site_service.list_all_keys().await;
    LandscapeApiResp::success(result)
}

async fn refresh_geo_site_cache(State(state): State<LandscapeApp>) -> LandscapeApiResult<()> {
    state.geo_site_service.refresh(true).await;
    LandscapeApiResp::success(())
}

async fn get_geo_sites(
    State(state): State<LandscapeApp>,
    Query(q): Query<QueryGeoDomainConfig>,
) -> LandscapeApiResult<Vec<GeoSiteSourceConfig>> {
    let result = state.geo_site_service.query_geo_by_name(q.name).await;
    LandscapeApiResp::success(result)
}

async fn get_geo_rule(
    State(state): State<LandscapeApp>,
    Path(id): Path<ConfigId>,
) -> LandscapeApiResult<GeoSiteSourceConfig> {
    let result = state.geo_site_service.find_by_id(id).await;
    if let Some(config) = result {
        LandscapeApiResp::success(config)
    } else {
        Err(LandscapeApiError::NotFound(format!("Dns Rule id: {:?}", id)))
    }
}

async fn add_geo_site(
    State(state): State<LandscapeApp>,
    Json(dns_rule): Json<GeoSiteSourceConfig>,
) -> LandscapeApiResult<GeoSiteSourceConfig> {
    let result = state.geo_site_service.set(dns_rule).await;
    LandscapeApiResp::success(result)
}

async fn add_many_geo_sites(
    State(state): State<LandscapeApp>,
    Json(rules): Json<Vec<GeoSiteSourceConfig>>,
) -> LandscapeApiResult<()> {
    state.geo_site_service.set_list(rules).await;
    LandscapeApiResp::success(())
}

async fn del_geo_site(
    State(state): State<LandscapeApp>,
    Path(id): Path<ConfigId>,
) -> LandscapeApiResult<()> {
    state.geo_site_service.delete(id).await;
    LandscapeApiResp::success(())
}

// curl -vvv -k -X POST https://localhost:6443/api/src/config/geo_sites/test2/update_by_upload
// -H "Authorization: Bearer $(cat ../.landscape-router/landscape_api_token)"
//  -F "file=@../.landscape-router/geosite.dat1"
async fn update_by_upload(
    State(state): State<LandscapeApp>,
    Path(name): Path<String>,
    mut multipart: Multipart,
) -> LandscapeApiResult<()> {
    tracing::info!("Got upload request for: {}", name);

    let file = multipart.next_field().await;
    let Ok(Some(field)) = file else {
        return Err(LandscapeApiError::BadRequest("geo site file not found".to_string()));
    };

    let Ok(bytes) = field.bytes().await else {
        return Err(LandscapeApiError::BadRequest("geo site file read error".to_string()));
    };

    state.geo_site_service.update_geo_config_by_bytes(name, bytes).await;

    LandscapeApiResp::success(())
}
