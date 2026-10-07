//! Routes attached to servers, and DNS deletes that still need a retry.

use chrono::{DateTime, Utc};
use shared::response::DisplayError;
use sqlx::{PgPool, Row};
use std::sync::Arc;
use uuid::Uuid;

pub struct Route {
    pub uuid: Uuid,
    pub server_uuid: Uuid,
    pub kind: String,
    pub domain: String,
    pub allowed_domain_uuid: Option<Uuid>,
    pub port: i32,
    pub record_type: Option<String>,
    pub record_content: Option<String>,
    pub cloudflare_zone_id: Option<String>,
    pub cloudflare_record_id: Option<String>,
    pub created: DateTime<Utc>,
}

pub struct AdminRoute {
    pub route: Route,
    pub server_name: String,
}

pub struct CleanupTask {
    pub uuid: Uuid,
    pub zone_id: String,
    pub record_id: String,
    pub token_encrypted: String,
    pub attempts: i32,
}

fn unique(err: &sqlx::Error) -> bool {
    err.as_database_error()
        .and_then(|error| error.code())
        .is_some_and(|code| code == "23505")
}

fn map_route(row: &sqlx::postgres::PgRow) -> Result<Route, sqlx::Error> {
    Ok(Route {
        uuid: row.try_get("uuid")?,
        server_uuid: row.try_get("server_uuid")?,
        kind: row.try_get("kind")?,
        domain: row.try_get("domain")?,
        allowed_domain_uuid: row.try_get("allowed_domain_uuid")?,
        port: row.try_get("port")?,
        record_type: row.try_get("record_type")?,
        record_content: row.try_get("record_content")?,
        cloudflare_zone_id: row.try_get("cloudflare_zone_id")?,
        cloudflare_record_id: row.try_get("cloudflare_record_id")?,
        created: row.try_get("created")?,
    })
}

pub async fn routes_for_server(
    pool: &PgPool,
    server_uuid: Uuid,
) -> Result<Vec<Route>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT uuid, server_uuid, kind, domain, allowed_domain_uuid, port, \
         record_type, record_content, cloudflare_zone_id, cloudflare_record_id, created \
         FROM dev_dawid_caddy_routes WHERE server_uuid = $1 ORDER BY created, uuid",
    )
    .bind(server_uuid)
    .fetch_all(pool)
    .await?;

    rows.iter().map(map_route).collect()
}

pub async fn route_for_server(
    pool: &PgPool,
    server_uuid: Uuid,
    uuid: Uuid,
) -> Result<Option<Route>, sqlx::Error> {
    let row = sqlx::query(
        "SELECT uuid, server_uuid, kind, domain, allowed_domain_uuid, port, \
         record_type, record_content, cloudflare_zone_id, cloudflare_record_id, created \
         FROM dev_dawid_caddy_routes WHERE server_uuid = $1 AND uuid = $2",
    )
    .bind(server_uuid)
    .bind(uuid)
    .fetch_optional(pool)
    .await?;

    row.as_ref().map(map_route).transpose()
}

pub async fn count_for_server(pool: &PgPool, server_uuid: Uuid) -> Result<i64, sqlx::Error> {
    let row =
        sqlx::query("SELECT COUNT(*) AS count FROM dev_dawid_caddy_routes WHERE server_uuid = $1")
            .bind(server_uuid)
            .fetch_one(pool)
            .await?;
    row.try_get("count")
}

pub async fn count_for_allowed_domain(
    pool: &PgPool,
    allowed_domain_uuid: Uuid,
) -> Result<i64, sqlx::Error> {
    let row = sqlx::query(
        "SELECT COUNT(*) AS count FROM dev_dawid_caddy_routes WHERE allowed_domain_uuid = $1",
    )
    .bind(allowed_domain_uuid)
    .fetch_one(pool)
    .await?;
    row.try_get("count")
}

pub async fn domain_taken(pool: &PgPool, domain: &str) -> Result<bool, sqlx::Error> {
    let row = sqlx::query(
        "SELECT EXISTS(SELECT 1 FROM dev_dawid_caddy_routes WHERE domain = $1) AS taken",
    )
    .bind(domain)
    .fetch_one(pool)
    .await?;
    row.try_get("taken")
}

pub struct NewRoute<'a> {
    pub server_uuid: Uuid,
    pub kind: &'a str,
    pub domain: &'a str,
    pub parent_domain: Option<&'a str>,
    pub allowed_domain_uuid: Option<Uuid>,
    pub port: i32,
    pub record_type: Option<&'a str>,
    pub record_content: Option<&'a str>,
    pub cloudflare_zone_id: Option<&'a str>,
    pub cloudflare_record_id: Option<&'a str>,
}

pub async fn insert_route(pool: &PgPool, route: NewRoute<'_>) -> Result<Uuid, anyhow::Error> {
    let result = sqlx::query(
        r#"
        INSERT INTO dev_dawid_caddy_routes (
            server_uuid, kind, domain, parent_domain, allowed_domain_uuid, port,
            record_type, record_content, cloudflare_zone_id, cloudflare_record_id
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
        RETURNING uuid
        "#,
    )
    .bind(route.server_uuid)
    .bind(route.kind)
    .bind(route.domain)
    .bind(route.parent_domain)
    .bind(route.allowed_domain_uuid)
    .bind(route.port)
    .bind(route.record_type)
    .bind(route.record_content)
    .bind(route.cloudflare_zone_id)
    .bind(route.cloudflare_record_id)
    .fetch_one(pool)
    .await;

    match result {
        Ok(row) => Ok(row.try_get("uuid")?),
        Err(err) if unique(&err) => Err(DisplayError::new("that domain is already in use").into()),
        Err(err) => Err(err.into()),
    }
}

pub async fn update_port(pool: &PgPool, uuid: Uuid, port: i32) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE dev_dawid_caddy_routes SET port = $2 WHERE uuid = $1")
        .bind(uuid)
        .bind(port)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn delete_route(pool: &PgPool, uuid: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM dev_dawid_caddy_routes WHERE uuid = $1")
        .bind(uuid)
        .execute(pool)
        .await?;
    Ok(())
}

pub fn like_pattern(search: &str) -> String {
    let mut pattern = String::from('%');
    for character in search.chars() {
        if matches!(character, '%' | '_' | '\\') {
            pattern.push('\\');
        }
        pattern.push(character);
    }
    pattern.push('%');
    pattern
}

pub async fn admin_routes(
    pool: &PgPool,
    page: i64,
    per_page: i64,
    search: Option<&str>,
) -> Result<(i64, Vec<AdminRoute>), sqlx::Error> {
    let offset = (page - 1) * per_page;
    let pattern = search.map(like_pattern);
    let rows = sqlx::query(
        "SELECT uuid, server_uuid, kind, domain, allowed_domain_uuid, port, \
         record_type, record_content, cloudflare_zone_id, cloudflare_record_id, created, \
         servers.name AS server_name, COUNT(*) OVER() AS total_count \
         FROM dev_dawid_caddy_routes \
         JOIN servers ON servers.uuid = dev_dawid_caddy_routes.server_uuid \
         WHERE ($1::text IS NULL OR dev_dawid_caddy_routes.domain ILIKE $1 ESCAPE '\\') \
         ORDER BY dev_dawid_caddy_routes.created DESC \
         LIMIT $2 OFFSET $3",
    )
    .bind(pattern)
    .bind(per_page)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    let total = rows
        .first()
        .map(|row| row.try_get("total_count"))
        .transpose()?
        .unwrap_or(0);
    let routes = rows
        .iter()
        .map(|row| {
            Ok(AdminRoute {
                route: map_route(row)?,
                server_name: row.try_get("server_name")?,
            })
        })
        .collect::<Result<Vec<_>, sqlx::Error>>()?;

    Ok((total, routes))
}

pub async fn route_by_uuid(pool: &PgPool, uuid: Uuid) -> Result<Option<Route>, sqlx::Error> {
    let row = sqlx::query(
        "SELECT uuid, server_uuid, kind, domain, allowed_domain_uuid, port, \
         record_type, record_content, cloudflare_zone_id, cloudflare_record_id, created \
         FROM dev_dawid_caddy_routes WHERE uuid = $1",
    )
    .bind(uuid)
    .fetch_optional(pool)
    .await?;
    row.as_ref().map(map_route).transpose()
}

pub async fn insert_cleanup(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    zone_id: &str,
    record_id: &str,
    token_encrypted: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO dev_dawid_caddy_cleanup (zone_id, record_id, token_encrypted)
        VALUES ($1, $2, $3)
        "#,
    )
    .bind(zone_id)
    .bind(record_id)
    .bind(token_encrypted)
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

pub async fn due_cleanup(pool: &PgPool) -> Result<Vec<CleanupTask>, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT uuid, zone_id, record_id, token_encrypted, attempts
        FROM dev_dawid_caddy_cleanup
        WHERE next_attempt <= now()
        ORDER BY next_attempt
        LIMIT 20
        "#,
    )
    .fetch_all(pool)
    .await?;

    rows.iter()
        .map(|row| {
            Ok(CleanupTask {
                uuid: row.try_get("uuid")?,
                zone_id: row.try_get("zone_id")?,
                record_id: row.try_get("record_id")?,
                token_encrypted: row.try_get("token_encrypted")?,
                attempts: row.try_get("attempts")?,
            })
        })
        .collect()
}

pub async fn delete_cleanup(pool: &PgPool, uuid: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM dev_dawid_caddy_cleanup WHERE uuid = $1")
        .bind(uuid)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn postpone_cleanup(
    pool: &PgPool,
    uuid: Uuid,
    attempts: i32,
    error: &str,
) -> Result<(), sqlx::Error> {
    let minutes = i32::try_from((2_i64).pow(attempts.clamp(0, 8) as u32).min(360)).unwrap_or(360);
    sqlx::query(
        r#"
        UPDATE dev_dawid_caddy_cleanup
        SET attempts = $2, last_error = $3, next_attempt = now() + make_interval(mins => $4)
        WHERE uuid = $1
        "#,
    )
    .bind(uuid)
    .bind(attempts)
    .bind(error)
    .bind(minutes)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn pending_cleanup(pool: &PgPool) -> Result<i64, sqlx::Error> {
    let row = sqlx::query("SELECT COUNT(*) AS count FROM dev_dawid_caddy_cleanup")
        .fetch_one(pool)
        .await?;
    row.try_get("count")
}

pub fn pool(database: &Arc<shared::database::Database>) -> &PgPool {
    database.read()
}

pub fn write_pool(database: &Arc<shared::database::Database>) -> &PgPool {
    database.write()
}
