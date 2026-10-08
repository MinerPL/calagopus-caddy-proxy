//! Create, retarget and remove a domain, then rewrite the server's Caddy labels.

use crate::{
    cloudflare,
    db::{self, NewRoute, Route},
    domain,
    labels::{self, LabelSource},
    model,
    settings::{self, AllowedDomain, ExtensionSettingsData},
};
use axum::http::StatusCode;
use compact_str::CompactString;
use indexmap::IndexMap;
use shared::{
    models::{
        server::{Server, UpdateServerOptions},
        ByUuid, UpdatableModel,
    },
    response::DisplayError,
    State,
};
use uuid::Uuid;

fn invalid(message: impl Into<String>) -> anyhow::Error {
    DisplayError::new(message.into())
        .with_status(StatusCode::BAD_REQUEST)
        .into()
}

fn label_sources(routes: &[Route]) -> Vec<LabelSource> {
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

pub async fn sync_labels(state: &State, server_uuid: Uuid) -> Result<(), anyhow::Error> {
    let routes = db::routes_for_server(db::pool(&state.database), server_uuid).await?;
    let mut server = Server::by_uuid(&state.database, server_uuid).await?;
    let merged = labels::merge(&server.labels, &label_sources(&routes)).map_err(invalid)?;

    let mut next = IndexMap::<CompactString, CompactString>::new();
    for (key, value) in merged {
        next.insert(key.into(), value.into());
    }

    server
        .update(
            state,
            UpdateServerOptions {
                labels: Some(next),
                ..Default::default()
            },
        )
        .await?;
    Ok(())
}

pub struct CreatedRoute {
    pub uuid: Uuid,
}

pub async fn create_subdomain(
    state: &State,
    server: &Server,
    settings: &ExtensionSettingsData,
    allowed: &AllowedDomain,
    name: &str,
    port: u16,
) -> Result<CreatedRoute, anyhow::Error> {
    ensure_capacity(state, server).await?;

    let name = domain::normalize_subdomain_name(name).map_err(invalid)?;
    let fqdn = domain::join_domain(&name, &allowed.domain).map_err(invalid)?;
    if db::domain_taken(db::pool(&state.database), &fqdn).await? {
        return Err(invalid("that domain is already in use"));
    }

    let (record_type, record_content) = match settings.record_for_node(server.node.uuid) {
        Ok(record) => record,
        Err(_) => {
            let node = server.node.fetch(&state.database).await?;
            crate::node_record::default_record(&node)
                .ok_or_else(|| invalid("set a public record for this server's node"))?
        }
    };
    if record_type == "CNAME" && record_content == fqdn {
        return Err(invalid("a cname record cannot point at itself"));
    }

    let (zone_id, record_id) = if allowed.cloudflare_enabled {
        if allowed.cloudflare_api_token.is_empty() || allowed.cloudflare_zone_id.trim().is_empty() {
            return Err(invalid(
                "cloudflare is enabled for this domain but the zone id or api token is missing",
            ));
        }
        let existing = cloudflare::address_records(
            allowed.cloudflare_zone_id.trim(),
            &allowed.cloudflare_api_token,
            &fqdn,
        )
        .await?;
        if !existing.is_empty() {
            return Err(invalid(cloudflare::stale_record_message(&fqdn, &existing)));
        }
        let id = cloudflare::create_record(
            allowed.cloudflare_zone_id.trim(),
            &allowed.cloudflare_api_token,
            &record_type,
            &fqdn,
            &record_content,
            allowed.cloudflare_proxied,
        )
        .await?;
        (Some(allowed.cloudflare_zone_id.trim()), Some(id))
    } else {
        (None, None)
    };

    let uuid = match db::insert_route(
        db::write_pool(&state.database),
        NewRoute {
            server_uuid: server.uuid,
            kind: "subdomain",
            domain: &fqdn,
            parent_domain: Some(&allowed.domain),
            allowed_domain_uuid: Some(allowed.uuid),
            port: i32::from(port),
            record_type: Some(&record_type),
            record_content: Some(&record_content),
            cloudflare_zone_id: zone_id,
            cloudflare_record_id: record_id.as_deref(),
        },
    )
    .await
    {
        Ok(uuid) => uuid,
        Err(err) => {
            if let Some(record_id) = record_id.as_deref()
                && let Some(zone_id) = zone_id
            {
                let _ =
                    cloudflare::delete_record(zone_id, &allowed.cloudflare_api_token, record_id)
                        .await;
            }
            return Err(err);
        }
    };

    if let Err(err) = sync_labels(state, server.uuid).await {
        let _ = db::delete_route(db::write_pool(&state.database), uuid).await;
        if let Some(record_id) = record_id.as_deref()
            && let Some(zone_id) = zone_id
        {
            let _ =
                cloudflare::delete_record(zone_id, &allowed.cloudflare_api_token, record_id).await;
        }
        return Err(err);
    }

    Ok(CreatedRoute { uuid })
}

pub async fn create_custom(
    state: &State,
    server: &Server,
    settings: &ExtensionSettingsData,
    domain_name: &str,
    port: u16,
) -> Result<CreatedRoute, anyhow::Error> {
    if !settings.allow_custom_domains {
        return Err(invalid("custom domains are disabled"));
    }
    ensure_capacity(state, server).await?;

    let fqdn = domain::normalize_hostname(domain_name)
        .ok_or_else(|| invalid("enter a domain name like play.example.com"))?;
    if settings
        .domains
        .iter()
        .any(|domain| domain::is_under_domain(&fqdn, &domain.domain))
    {
        return Err(invalid(
            "that name belongs to an allowed domain; create it as a subdomain so dns can be managed",
        ));
    }
    if db::domain_taken(db::pool(&state.database), &fqdn).await? {
        return Err(invalid("that domain is already in use"));
    }

    let uuid = db::insert_route(
        db::write_pool(&state.database),
        NewRoute {
            server_uuid: server.uuid,
            kind: "custom",
            domain: &fqdn,
            parent_domain: None,
            allowed_domain_uuid: None,
            port: i32::from(port),
            record_type: None,
            record_content: None,
            cloudflare_zone_id: None,
            cloudflare_record_id: None,
        },
    )
    .await?;

    if let Err(err) = sync_labels(state, server.uuid).await {
        let _ = db::delete_route(db::write_pool(&state.database), uuid).await;
        return Err(err);
    }

    Ok(CreatedRoute { uuid })
}

pub async fn change_port(state: &State, route: &Route, port: u16) -> Result<(), anyhow::Error> {
    let previous = route.port;
    db::update_port(db::write_pool(&state.database), route.uuid, i32::from(port)).await?;
    if let Err(err) = sync_labels(state, route.server_uuid).await {
        let _ = db::update_port(db::write_pool(&state.database), route.uuid, previous).await;
        return Err(err);
    }
    Ok(())
}

pub async fn remove_route(state: &State, route: &Route) -> Result<(), anyhow::Error> {
    let pool = db::pool(&state.database);
    let mut remaining = db::routes_for_server(pool, route.server_uuid).await?;
    remaining.retain(|item| item.uuid != route.uuid);

    let mut server = Server::by_uuid(&state.database, route.server_uuid).await?;
    let previous = server.labels.clone();
    let merged = labels::merge(&server.labels, &label_sources(&remaining)).map_err(invalid)?;
    let mut next = IndexMap::<CompactString, CompactString>::new();
    for (key, value) in merged {
        next.insert(key.into(), value.into());
    }
    server
        .update(
            state,
            UpdateServerOptions {
                labels: Some(next),
                ..Default::default()
            },
        )
        .await?;

    if let (Some(zone_id), Some(record_id)) =
        (&route.cloudflare_zone_id, &route.cloudflare_record_id)
    {
        let settings = settings::load(state).await?;
        let token = route
            .allowed_domain_uuid
            .and_then(|uuid| settings.domain(uuid))
            .map(|domain| domain.cloudflare_api_token.clone())
            .filter(|token| !token.is_empty());
        let Some(token) = token else {
            restore_labels(state, route.server_uuid, previous).await;
            return Err(invalid(
                "the cloudflare token for this domain is missing, so the dns record was kept",
            ));
        };
        if let Err(err) = cloudflare::delete_record(zone_id, &token, record_id).await {
            restore_labels(state, route.server_uuid, previous).await;
            return Err(err);
        }
    }

    if let Err(err) = db::delete_route(db::write_pool(&state.database), route.uuid).await {
        restore_labels(state, route.server_uuid, previous).await;
        return Err(err.into());
    }
    Ok(())
}

async fn restore_labels(
    state: &State,
    server_uuid: Uuid,
    previous: IndexMap<CompactString, CompactString>,
) {
    if let Ok(mut server) = Server::by_uuid(&state.database, server_uuid).await {
        let _ = server
            .update(
                state,
                UpdateServerOptions {
                    labels: Some(previous),
                    ..Default::default()
                },
            )
            .await;
    }
}

async fn ensure_capacity(state: &State, server: &Server) -> Result<(), anyhow::Error> {
    let limit = model::caddy_domains(server)?;
    if limit <= 0 {
        return Err(invalid("caddy domains are disabled for this server"));
    }
    let count = db::count_for_server(db::pool(&state.database), server.uuid).await?;
    if count >= i64::from(limit) {
        return Err(invalid(format!(
            "this server already has {count} of {limit} caddy domains"
        )));
    }
    Ok(())
}

/// Deletes Cloudflare records for every route on a server that is being removed.
/// A failed DNS delete is queued, with the token encrypted, and retried later.
pub async fn on_server_deleted(
    state: &State,
    server_uuid: Uuid,
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
) -> Result<(), anyhow::Error> {
    let routes = db::routes_for_server(db::pool(&state.database), server_uuid).await?;
    let settings = settings::load(state).await?;

    for route in routes {
        let (Some(zone_id), Some(record_id)) =
            (&route.cloudflare_zone_id, &route.cloudflare_record_id)
        else {
            continue;
        };
        let Some(token) = route
            .allowed_domain_uuid
            .and_then(|uuid| settings.domain(uuid))
            .map(|domain| domain.cloudflare_api_token.clone())
            .filter(|token| !token.is_empty())
        else {
            tracing::warn!(
                domain = %route.domain,
                "caddy: server deleted but the cloudflare token is gone, so the dns record was left in place"
            );
            continue;
        };

        if let Err(err) = cloudflare::delete_record(zone_id, &token, record_id).await {
            tracing::warn!(domain = %route.domain, "caddy: dns delete failed, queueing retry: {err:?}");
            let encrypted = state.database.encrypt_base64(token).await?;
            db::insert_cleanup(transaction, zone_id, record_id, &encrypted).await?;
        }
    }

    Ok(())
}
