use super::State;
use crate::{
    db::{self, Route},
    labels::{self, LabelSource},
    settings::ExtensionSettingsData,
};
use shared::{models::server::Server, response::ApiResponse};
use utoipa_axum::{router::OpenApiRouter, routes};
use uuid::Uuid;

fn sources(routes: &[Route]) -> Vec<LabelSource> {
    routes
        .iter()
        .filter_map(|route| {
            u16::try_from(route.port).ok().map(|port| LabelSource {
                domain: route.domain.clone(),
                port,
            })
        })
        .collect()
}

fn api_route(route: Route) -> ApiRoute {
    ApiRoute {
        dns: if route.cloudflare_record_id.is_some() {
            "cloudflare"
        } else {
            "manual"
        }
        .to_string(),
        uuid: route.uuid,
        kind: route.kind,
        domain: route.domain,
        port: route.port,
        record_type: route.record_type,
        record_content: route.record_content,
        created: route.created,
    }
}

fn api_labels(routes: &[Route]) -> Vec<ApiLabel> {
    labels::planned(&sources(routes))
        .unwrap_or_default()
        .into_iter()
        .map(|(key, value)| ApiLabel { key, value })
        .collect()
}

#[derive(utoipa::ToSchema, serde::Serialize)]
struct ApiLabel {
    key: String,
    value: String,
}

#[derive(utoipa::ToSchema, serde::Serialize)]
struct ApiRoute {
    uuid: Uuid,
    kind: String,
    domain: String,
    port: i32,
    record_type: Option<String>,
    record_content: Option<String>,
    dns: String,
    created: chrono::DateTime<chrono::Utc>,
}

#[derive(utoipa::ToSchema, serde::Serialize)]
struct PublicDomain {
    uuid: Uuid,
    domain: String,
    record_type: String,
    record_content: String,
    cloudflare: bool,
}

fn public_domains(settings: &ExtensionSettingsData) -> Vec<PublicDomain> {
    settings
        .domains
        .iter()
        .map(|domain| PublicDomain {
            cloudflare: domain.cloudflare_enabled,
            uuid: domain.uuid,
            domain: domain.domain.clone(),
            record_type: domain.record_type.clone(),
            record_content: domain.record_content.clone(),
        })
        .collect()
}

async fn find_route(state: &State, server: &Server, uuid: Uuid) -> Result<Route, ApiResponse> {
    db::route_for_server(db::pool(&state.database), server.uuid, uuid)
        .await?
        .ok_or_else(|| super::not_found("domain"))
}

fn parse_port(port: i32) -> Result<u16, ApiResponse> {
    u16::try_from(port)
        .ok()
        .filter(|port| *port >= 1)
        .ok_or_else(|| ApiResponse::error("port must be between 1 and 65535"))
}

mod list {
    use super::{api_labels, api_route, public_domains, ApiLabel, ApiRoute, PublicDomain};
    use crate::{db, model, settings};
    use serde::Serialize;
    use shared::{
        models::{server::GetServer, user::GetPermissionManager},
        response::{ApiResponse, ApiResponseResult},
        GetState,
    };
    use utoipa::ToSchema;

    #[derive(ToSchema, Serialize)]
    struct Response {
        routes: Vec<ApiRoute>,
        labels: Vec<ApiLabel>,
        limit: i32,
        allow_custom_domains: bool,
        allowed_domains: Vec<PublicDomain>,
    }

    #[utoipa::path(get, path = "/", responses(
        (status = OK, body = inline(Response)),
    ), params(("server" = uuid::Uuid, description = "The server ID")))]
    pub async fn route(
        state: GetState,
        permissions: GetPermissionManager,
        server: GetServer,
    ) -> ApiResponseResult {
        permissions.has_server_permission("caddy.read")?;

        let settings = settings::load(&state).await?;
        let routes = db::routes_for_server(db::pool(&state.database), server.uuid).await?;

        ApiResponse::new_serialized(Response {
            labels: api_labels(&routes),
            limit: model::caddy_domains(&server)?,
            allow_custom_domains: settings.allow_custom_domains,
            allowed_domains: public_domains(&settings),
            routes: routes.into_iter().map(api_route).collect(),
        })
        .ok()
    }
}

mod create {
    use super::{api_route, find_route, parse_port, ApiRoute};
    use crate::{service, settings};
    use garde::Validate;
    use serde::{Deserialize, Serialize};
    use shared::{
        models::{
            server::{GetServer, GetServerActivityLogger},
            user::GetPermissionManager,
        },
        response::{ApiResponse, ApiResponseResult},
        ApiError, GetState,
    };
    use utoipa::ToSchema;

    #[derive(ToSchema, Validate, Deserialize)]
    pub struct Payload {
        /// `subdomain` of an allowed domain, or a `custom` domain.
        #[garde(length(chars, min = 1, max = 16))]
        kind: String,
        #[garde(skip)]
        domain_uuid: Option<uuid::Uuid>,
        /// Subdomain label, such as `play` or `mc.play`.
        #[garde(length(chars, max = 253))]
        name: Option<String>,
        /// Full custom domain.
        #[garde(length(chars, max = 253))]
        domain: Option<String>,
        /// Port the process listens on inside the container.
        #[garde(range(min = 1, max = 65535))]
        port: i32,
    }

    #[derive(ToSchema, Serialize)]
    struct Response {
        route: ApiRoute,
    }

    #[utoipa::path(post, path = "/", responses(
        (status = OK, body = inline(Response)),
        (status = BAD_REQUEST, body = ApiError),
    ), params(("server" = uuid::Uuid, description = "The server ID")), request_body = inline(Payload))]
    pub async fn route(
        state: GetState,
        permissions: GetPermissionManager,
        server: GetServer,
        activity_logger: GetServerActivityLogger,
        shared::Payload(data): shared::Payload<Payload>,
    ) -> ApiResponseResult {
        if let Some(response) = crate::routes::validation_error(&data) {
            return response.ok();
        }
        permissions.has_server_permission("caddy.create")?;
        let port = parse_port(data.port)?;

        let settings = settings::load(&state).await?;
        let created = match data.kind.as_str() {
            "subdomain" => {
                let domain_uuid = data
                    .domain_uuid
                    .ok_or_else(|| ApiResponse::error("choose a domain"))?;
                let allowed = settings
                    .domain(domain_uuid)
                    .ok_or_else(|| ApiResponse::error("that domain is not allowed"))?;
                let name = data
                    .name
                    .as_deref()
                    .filter(|name| !name.trim().is_empty())
                    .ok_or_else(|| ApiResponse::error("enter a subdomain name"))?;
                service::create_subdomain(&state, &server, allowed, name, port).await?
            }
            "custom" => {
                let domain = data
                    .domain
                    .as_deref()
                    .filter(|domain| !domain.trim().is_empty())
                    .ok_or_else(|| ApiResponse::error("enter a domain name"))?;
                service::create_custom(&state, &server, &settings, domain, port).await?
            }
            _ => return ApiResponse::error("kind must be subdomain or custom").ok(),
        };

        let route = find_route(&state, &server, created.uuid).await?;
        activity_logger
            .log(
                "server:caddy.create",
                serde_json::json!({
                    "domain": route.domain,
                    "kind": route.kind,
                    "port": route.port,
                    "dns": route.cloudflare_record_id.is_some(),
                }),
            )
            .await;

        ApiResponse::new_serialized(Response {
            route: api_route(route),
        })
        .ok()
    }
}

mod update {
    use super::{api_route, find_route, parse_port, ApiRoute};
    use crate::service;
    use axum::extract::Path;
    use garde::Validate;
    use serde::{Deserialize, Serialize};
    use shared::{
        models::{
            server::{GetServer, GetServerActivityLogger},
            user::GetPermissionManager,
        },
        response::{ApiResponse, ApiResponseResult},
        ApiError, GetState,
    };
    use utoipa::ToSchema;

    #[derive(ToSchema, Validate, Deserialize)]
    pub struct Payload {
        #[garde(range(min = 1, max = 65535))]
        port: i32,
    }

    #[derive(ToSchema, Serialize)]
    struct Response {
        route: ApiRoute,
    }

    #[utoipa::path(patch, path = "/", responses(
        (status = OK, body = inline(Response)),
        (status = BAD_REQUEST, body = ApiError),
    ), params(
        ("server" = uuid::Uuid, description = "The server ID"),
        ("route" = uuid::Uuid, description = "The domain ID"),
    ), request_body = inline(Payload))]
    pub async fn route(
        state: GetState,
        permissions: GetPermissionManager,
        server: GetServer,
        activity_logger: GetServerActivityLogger,
        Path((_server, route_uuid)): Path<(uuid::Uuid, uuid::Uuid)>,
        shared::Payload(data): shared::Payload<Payload>,
    ) -> ApiResponseResult {
        if let Some(response) = crate::routes::validation_error(&data) {
            return response.ok();
        }
        permissions.has_server_permission("caddy.update")?;

        let existing = find_route(&state, &server, route_uuid).await?;
        let port = parse_port(data.port)?;
        service::change_port(&state, &existing, port).await?;

        let route = find_route(&state, &server, route_uuid).await?;
        activity_logger
            .log(
                "server:caddy.update",
                serde_json::json!({
                    "domain": route.domain,
                    "port": route.port,
                }),
            )
            .await;

        ApiResponse::new_serialized(Response {
            route: api_route(route),
        })
        .ok()
    }
}

mod delete {
    use super::find_route;
    use crate::service;
    use axum::extract::Path;
    use serde::Serialize;
    use shared::{
        models::{
            server::{GetServer, GetServerActivityLogger},
            user::GetPermissionManager,
        },
        response::{ApiResponse, ApiResponseResult},
        GetState,
    };
    use utoipa::ToSchema;

    #[derive(ToSchema, Serialize)]
    struct Response {}

    #[utoipa::path(delete, path = "/", responses(
        (status = OK, body = inline(Response)),
    ), params(
        ("server" = uuid::Uuid, description = "The server ID"),
        ("route" = uuid::Uuid, description = "The domain ID"),
    ))]
    pub async fn route(
        state: GetState,
        permissions: GetPermissionManager,
        server: GetServer,
        activity_logger: GetServerActivityLogger,
        Path((_server, route_uuid)): Path<(uuid::Uuid, uuid::Uuid)>,
    ) -> ApiResponseResult {
        permissions.has_server_permission("caddy.delete")?;

        let existing = find_route(&state, &server, route_uuid).await?;
        let domain = existing.domain.clone();
        service::remove_route(&state, &existing).await?;

        activity_logger
            .log(
                "server:caddy.delete",
                serde_json::json!({ "domain": domain }),
            )
            .await;

        ApiResponse::new_serialized(Response {}).ok()
    }
}

pub fn router(state: &State) -> OpenApiRouter<State> {
    OpenApiRouter::new()
        .routes(routes!(list::route))
        .routes(routes!(create::route))
        .nest("/{route}", update_router(state))
        .with_state(state.clone())
}

fn update_router(state: &State) -> OpenApiRouter<State> {
    OpenApiRouter::new()
        .routes(routes!(update::route))
        .routes(routes!(delete::route))
        .with_state(state.clone())
}
