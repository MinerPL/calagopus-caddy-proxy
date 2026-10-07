use super::State;
use crate::{
    domain,
    settings::{AllowedDomain, ExtensionSettingsData, NodePoint},
};
use shared::response::DisplayError;
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;

fn invalid(message: impl Into<String>) -> anyhow::Error {
    DisplayError::new(message.into()).into()
}

fn prepare_domains(
    current: &ExtensionSettingsData,
    incoming: Vec<DomainPayload>,
) -> Result<Vec<AllowedDomain>, anyhow::Error> {
    let mut prepared = Vec::with_capacity(incoming.len());
    let mut seen = Vec::new();

    for domain in incoming {
        let hostname = domain::normalize_hostname(&domain.domain)
            .ok_or_else(|| invalid(format!("`{}` is not a valid domain", domain.domain)))?;
        if seen.iter().any(|existing: &String| existing == &hostname) {
            return Err(invalid(format!("`{hostname}` is listed more than once")));
        }
        seen.push(hostname.clone());

        let uuid = domain.uuid.unwrap_or_else(Uuid::new_v4);
        let previous = current
            .domains
            .iter()
            .find(|existing| existing.uuid == uuid);
        let token = match domain.cloudflare_api_token.as_deref().map(str::trim) {
            Some(token) if !token.is_empty() => token.to_string(),
            _ => previous
                .map(|existing| existing.cloudflare_api_token.clone())
                .unwrap_or_default(),
        };

        if domain.cloudflare_enabled {
            if domain.cloudflare_zone_id.trim().is_empty() {
                return Err(invalid(format!("`{hostname}` needs a cloudflare zone id")));
            }
            if token.is_empty() {
                return Err(invalid(format!(
                    "`{hostname}` needs a cloudflare api token"
                )));
            }
        }

        prepared.push(AllowedDomain {
            uuid,
            domain: hostname,
            cloudflare_enabled: domain.cloudflare_enabled,
            cloudflare_zone_id: domain.cloudflare_zone_id.trim().to_string(),
            cloudflare_api_token: token,
            cloudflare_proxied: domain.cloudflare_proxied,
        });
    }

    Ok(prepared)
}

fn prepare_points(
    incoming: Vec<NodePointPayload>,
    nodes: &[ApiNode],
) -> Result<Vec<NodePoint>, anyhow::Error> {
    let mut prepared = Vec::with_capacity(nodes.len());

    for node in nodes {
        let point = incoming
            .iter()
            .find(|point| point.node_uuid == node.uuid)
            .ok_or_else(|| invalid(format!("`{}` needs a public record", node.name)))?;
        if incoming
            .iter()
            .filter(|point| point.node_uuid == node.uuid)
            .count()
            > 1
        {
            return Err(invalid(format!("`{}` is listed more than once", node.name)));
        }

        let content = point.record_content.trim();
        if content.is_empty() {
            return Err(invalid(format!("`{}` needs a public record", node.name)));
        }
        let (record_type, record_content) = domain::normalize_record(&point.record_type, content)
            .map_err(|err| invalid(format!("{}: {err}", node.name)))?;
        prepared.push(NodePoint {
            node_uuid: node.uuid,
            record_type,
            record_content,
        });
    }

    Ok(prepared)
}

async fn load_nodes(state: &State) -> Result<Vec<ApiNode>, anyhow::Error> {
    use shared::models::node::Node;

    let mut page = 1_i64;
    let mut nodes = Vec::new();
    loop {
        let batch = Node::all_with_pagination(&state.database, page, 100, None).await?;
        let total = batch.total;
        let count = batch.data.len();
        nodes.extend(batch.data.into_iter().map(|node| ApiNode {
            domain: crate::node_record::hostname(&node),
            uuid: node.uuid,
            name: node.name.to_string(),
        }));
        if count == 0 || nodes.len() as i64 >= total || page >= 20 {
            break;
        }
        page += 1;
    }
    nodes.sort_by(|left, right| left.name.to_lowercase().cmp(&right.name.to_lowercase()));
    Ok(nodes)
}

#[derive(utoipa::ToSchema, serde::Deserialize, garde::Validate)]
struct DomainPayload {
    #[garde(skip)]
    uuid: Option<Uuid>,
    #[garde(length(chars, min = 1, max = 253))]
    domain: String,
    #[garde(skip)]
    cloudflare_enabled: bool,
    #[garde(length(chars, max = 64))]
    cloudflare_zone_id: String,
    #[garde(length(chars, max = 255))]
    cloudflare_api_token: Option<String>,
    #[garde(skip)]
    cloudflare_proxied: bool,
}

#[derive(utoipa::ToSchema, serde::Deserialize, garde::Validate)]
struct NodePointPayload {
    #[garde(skip)]
    node_uuid: Uuid,
    #[garde(length(chars, min = 1, max = 8))]
    record_type: String,
    #[garde(length(chars, max = 253))]
    record_content: String,
}

#[derive(utoipa::ToSchema, serde::Serialize)]
struct ApiDomain {
    uuid: Uuid,
    domain: String,
    cloudflare_enabled: bool,
    cloudflare_zone_id: String,
    cloudflare_token_set: bool,
    cloudflare_proxied: bool,
}

#[derive(utoipa::ToSchema, serde::Serialize)]
struct ApiNode {
    uuid: Uuid,
    name: String,
    /// Hostname of the node, used to prefill a CNAME.
    domain: Option<String>,
}

#[derive(utoipa::ToSchema, serde::Serialize)]
struct ApiNodePoint {
    node_uuid: Uuid,
    record_type: String,
    record_content: String,
}

fn api_domain(domain: &AllowedDomain) -> ApiDomain {
    ApiDomain {
        cloudflare_token_set: !domain.cloudflare_api_token.is_empty(),
        uuid: domain.uuid,
        domain: domain.domain.clone(),
        cloudflare_enabled: domain.cloudflare_enabled,
        cloudflare_zone_id: domain.cloudflare_zone_id.clone(),
        cloudflare_proxied: domain.cloudflare_proxied,
    }
}

fn api_points(settings: &ExtensionSettingsData) -> Vec<ApiNodePoint> {
    settings
        .node_points
        .iter()
        .map(|point| ApiNodePoint {
            node_uuid: point.node_uuid,
            record_type: point.record_type.clone(),
            record_content: point.record_content.clone(),
        })
        .collect()
}

mod get {
    use super::{api_domain, api_points, load_nodes, ApiDomain, ApiNode, ApiNodePoint};
    use crate::{db, settings};
    use serde::Serialize;
    use shared::{
        models::user::GetPermissionManager,
        response::{ApiResponse, ApiResponseResult},
        GetState,
    };
    use utoipa::ToSchema;

    #[derive(ToSchema, Serialize)]
    struct Response {
        allow_custom_domains: bool,
        default_limit: i32,
        domains: Vec<ApiDomain>,
        nodes: Vec<ApiNode>,
        node_points: Vec<ApiNodePoint>,
        pending_cleanup: i64,
    }

    #[utoipa::path(get, path = "/", responses((status = OK, body = inline(Response))))]
    pub async fn route(state: GetState, permissions: GetPermissionManager) -> ApiResponseResult {
        permissions.has_admin_permission("caddy.read")?;
        let settings = settings::load(&state).await?;
        ApiResponse::new_serialized(Response {
            allow_custom_domains: settings.allow_custom_domains,
            default_limit: settings.default_limit,
            pending_cleanup: db::pending_cleanup(db::pool(&state.database)).await?,
            nodes: load_nodes(&state).await?,
            node_points: api_points(&settings),
            domains: settings.domains.iter().map(api_domain).collect(),
        })
        .ok()
    }
}

mod put {
    use super::{
        api_domain, api_points, load_nodes, prepare_domains, prepare_points, ApiNode, DomainPayload,
        NodePointPayload,
    };
    use crate::{db, settings};
    use garde::Validate;
    use serde::{Deserialize, Serialize};
    use shared::{
        models::{admin_activity::GetAdminActivityLogger, user::GetPermissionManager},
        response::{ApiResponse, ApiResponseResult},
        ApiError, GetState,
    };
    use utoipa::ToSchema;

    #[derive(ToSchema, Validate, Deserialize)]
    pub struct Payload {
        #[garde(skip)]
        allow_custom_domains: bool,
        #[garde(range(min = 0, max = 1000))]
        default_limit: i32,
        #[garde(dive)]
        domains: Vec<DomainPayload>,
        #[garde(dive)]
        node_points: Vec<NodePointPayload>,
    }

    #[derive(ToSchema, Serialize)]
    struct Response {
        allow_custom_domains: bool,
        default_limit: i32,
        domains: Vec<super::ApiDomain>,
        nodes: Vec<ApiNode>,
        node_points: Vec<super::ApiNodePoint>,
        pending_cleanup: i64,
    }

    #[utoipa::path(put, path = "/", responses(
        (status = OK, body = inline(Response)),
        (status = BAD_REQUEST, body = ApiError),
    ), request_body = inline(Payload))]
    pub async fn route(
        state: GetState,
        permissions: GetPermissionManager,
        activity_logger: GetAdminActivityLogger,
        shared::Payload(data): shared::Payload<Payload>,
    ) -> ApiResponseResult {
        if let Some(response) = crate::routes::validation_error(&data) {
            return response.ok();
        }
        permissions.has_admin_permission("caddy.manage")?;

        let current = settings::load(&state).await?;
        let nodes = load_nodes(&state).await?;
        let domains = prepare_domains(&current, data.domains)?;
        let node_points = prepare_points(data.node_points, &nodes)?;

        for existing in &current.domains {
            let next = domains.iter().find(|domain| domain.uuid == existing.uuid);
            let still_used =
                db::count_for_allowed_domain(db::pool(&state.database), existing.uuid).await?;
            if still_used == 0 {
                continue;
            }
            match next {
                None => {
                    return ApiResponse::error(format!(
                        "{} still has {still_used} subdomains; delete them before removing the domain",
                        existing.domain
                    ))
                    .ok();
                }
                Some(next) if next.domain != existing.domain => {
                    return ApiResponse::error(format!(
                        "{} still has {still_used} subdomains; delete them before renaming the domain",
                        existing.domain
                    ))
                    .ok();
                }
                Some(_) => {}
            }
        }

        let pending_cleanup = db::pending_cleanup(db::pool(&state.database)).await?;
        let mut stored = state.settings.get_mut().await?;
        let extension: &mut settings::ExtensionSettingsData =
            stored.find_mut_extension_settings()?;
        extension.allow_custom_domains = data.allow_custom_domains;
        extension.default_limit = data.default_limit;
        extension.domains = domains;
        extension.node_points = node_points;
        let node_points = api_points(extension);
        let domains = extension.domains.iter().map(api_domain).collect();
        let response = Response {
            allow_custom_domains: extension.allow_custom_domains,
            default_limit: extension.default_limit,
            pending_cleanup,
            nodes,
            node_points,
            domains,
        };
        stored.save().await?;

        activity_logger
            .log(
                "settings:caddy.update",
                serde_json::json!({
                    "domains": response.domains.len(),
                    "node_points": response.node_points.len(),
                    "allow_custom_domains": response.allow_custom_domains,
                    "default_limit": response.default_limit,
                }),
            )
            .await;

        ApiResponse::new_serialized(response).ok()
    }
}

mod test_zone {
    use crate::{cloudflare, domain, settings};
    use garde::Validate;
    use serde::{Deserialize, Serialize};
    use shared::{
        models::user::GetPermissionManager,
        response::{ApiResponse, ApiResponseResult},
        ApiError, GetState,
    };
    use utoipa::ToSchema;
    use uuid::Uuid;

    #[derive(ToSchema, Validate, Deserialize)]
    pub struct Payload {
        #[garde(skip)]
        uuid: Option<Uuid>,
        #[garde(length(chars, min = 1, max = 253))]
        domain: String,
        #[garde(length(chars, min = 1, max = 64))]
        cloudflare_zone_id: String,
        #[garde(length(chars, max = 255))]
        cloudflare_api_token: Option<String>,
    }

    #[derive(ToSchema, Serialize)]
    struct Response {
        zone_name: String,
    }

    #[utoipa::path(post, path = "/", responses(
        (status = OK, body = inline(Response)),
        (status = BAD_REQUEST, body = ApiError),
    ), request_body = inline(Payload))]
    pub async fn route(
        state: GetState,
        permissions: GetPermissionManager,
        shared::Payload(data): shared::Payload<Payload>,
    ) -> ApiResponseResult {
        if let Some(response) = crate::routes::validation_error(&data) {
            return response.ok();
        }
        permissions.has_admin_permission("caddy.manage")?;

        let domain = domain::normalize_hostname(&data.domain)
            .ok_or_else(|| ApiResponse::error("enter a valid domain"))?;
        let settings = settings::load(&state).await?;
        let token = match data.cloudflare_api_token.as_deref().map(str::trim) {
            Some(token) if !token.is_empty() => token.to_string(),
            _ => data
                .uuid
                .and_then(|uuid| settings.domain(uuid))
                .map(|domain| domain.cloudflare_api_token.clone())
                .filter(|token| !token.is_empty())
                .ok_or_else(|| ApiResponse::error("enter a cloudflare api token"))?,
        };

        let zone_name =
            cloudflare::verify_zone(data.cloudflare_zone_id.trim(), &token, &domain).await?;
        ApiResponse::new_serialized(Response { zone_name }).ok()
    }
}

mod routes_list {
    use crate::db::{self, AdminRoute};
    use axum::extract::Query;
    use serde::Serialize;
    use shared::{
        models::{user::GetPermissionManager, Pagination, PaginationParamsWithSearch},
        response::{ApiResponse, ApiResponseResult},
        GetState,
    };
    use utoipa::ToSchema;
    use uuid::Uuid;

    #[derive(ToSchema, Serialize)]
    struct ApiAdminRoute {
        uuid: Uuid,
        server_uuid: Uuid,
        server_name: String,
        kind: String,
        domain: String,
        port: i32,
        dns: String,
        created: chrono::DateTime<chrono::Utc>,
    }

    fn convert(route: AdminRoute) -> ApiAdminRoute {
        ApiAdminRoute {
            dns: if route.route.cloudflare_record_id.is_some() {
                "cloudflare"
            } else {
                "manual"
            }
            .to_string(),
            uuid: route.route.uuid,
            server_uuid: route.route.server_uuid,
            server_name: route.server_name,
            kind: route.route.kind,
            domain: route.route.domain,
            port: route.route.port,
            created: route.route.created,
        }
    }

    #[utoipa::path(get, path = "/", responses((status = OK, body = inline(Pagination<ApiAdminRoute>))))]
    pub async fn route(
        state: GetState,
        permissions: GetPermissionManager,
        Query(params): Query<PaginationParamsWithSearch>,
    ) -> ApiResponseResult {
        permissions.has_admin_permission("caddy.read")?;
        let (total, routes) = db::admin_routes(
            db::pool(&state.database),
            params.page,
            params.per_page,
            params.search.as_deref(),
        )
        .await?;

        ApiResponse::new_serialized(Pagination {
            total,
            per_page: params.per_page,
            page: params.page,
            data: routes.into_iter().map(convert).collect(),
        })
        .ok()
    }
}

mod routes_delete {
    use crate::{db, service};
    use axum::extract::Path;
    use serde::Serialize;
    use shared::{
        models::{admin_activity::GetAdminActivityLogger, user::GetPermissionManager},
        response::{ApiResponse, ApiResponseResult},
        GetState,
    };
    use utoipa::ToSchema;

    #[derive(ToSchema, Serialize)]
    struct Response {}

    #[utoipa::path(delete, path = "/", responses((status = OK, body = inline(Response))), params(
        ("route" = uuid::Uuid, description = "The domain ID"),
    ))]
    pub async fn route(
        state: GetState,
        permissions: GetPermissionManager,
        activity_logger: GetAdminActivityLogger,
        Path(route_uuid): Path<uuid::Uuid>,
    ) -> ApiResponseResult {
        permissions.has_admin_permission("caddy.manage")?;
        let route = db::route_by_uuid(db::pool(&state.database), route_uuid)
            .await?
            .ok_or_else(|| super::super::not_found("domain"))?;
        let domain = route.domain.clone();
        let server_uuid = route.server_uuid;
        service::remove_route(&state, &route).await?;

        activity_logger
            .log(
                "settings:caddy.delete",
                serde_json::json!({
                    "domain": domain,
                    "server": server_uuid,
                }),
            )
            .await;

        ApiResponse::new_serialized(Response {}).ok()
    }
}

pub fn router(state: &State) -> OpenApiRouter<State> {
    OpenApiRouter::new()
        .nest("/settings", settings_router(state))
        .nest("/domains/test", test_router(state))
        .nest("/routes", routes_router(state))
        .with_state(state.clone())
}

fn settings_router(state: &State) -> OpenApiRouter<State> {
    OpenApiRouter::new()
        .routes(routes!(get::route))
        .routes(routes!(put::route))
        .with_state(state.clone())
}

fn test_router(state: &State) -> OpenApiRouter<State> {
    OpenApiRouter::new()
        .routes(routes!(test_zone::route))
        .with_state(state.clone())
}

fn routes_router(state: &State) -> OpenApiRouter<State> {
    OpenApiRouter::new()
        .routes(routes!(routes_list::route))
        .nest("/{route}", route_router(state))
        .with_state(state.clone())
}

fn route_router(state: &State) -> OpenApiRouter<State> {
    OpenApiRouter::new()
        .routes(routes!(routes_delete::route))
        .with_state(state.clone())
}
