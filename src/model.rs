use garde::Validate;
use serde::{Deserialize, Serialize};
use shared::{
    database::DatabaseError,
    models::{
        server::Server, BaseModel, ModelExtension, ModelExtensionMapType, SafeModelExtension,
    },
};
use sqlx::{postgres::PgRow, Row};
use std::collections::BTreeMap;
use utoipa::ToSchema;

#[derive(Serialize, Deserialize)]
pub struct ServerExtensionData {
    pub caddy_domains: i32,
}

pub struct ServerExtension;

impl SafeModelExtension for ServerExtension {
    type Value = ServerExtensionData;

    fn name() -> &'static str {
        ServerExtension.extension_name()
    }
}

impl ModelExtension for ServerExtension {
    fn extension_name(&self) -> &'static str {
        "dev.minerpl.caddyproxy"
    }

    fn extended_columns(&self, prefix: &str) -> BTreeMap<&'static str, compact_str::CompactString> {
        BTreeMap::from([(
            "servers.caddy_domains",
            compact_str::format_compact!("{prefix}caddy_domains"),
        )])
    }

    fn map_extended(
        &self,
        prefix: &str,
        row: &PgRow,
    ) -> Result<ModelExtensionMapType, DatabaseError> {
        Ok(Box::new(ServerExtensionData {
            caddy_domains: row
                .try_get(compact_str::format_compact!("{prefix}caddy_domains").as_str())?,
        }))
    }
}

/// `caddy_domains` is optional so clients that do not know about this
/// extension leave the stored limit alone.
#[derive(ToSchema, Validate, Serialize, Deserialize)]
pub struct ExtendedApiServerFeatureLimits {
    #[garde(range(min = 0))]
    #[schema(minimum = 0)]
    pub caddy_domains: Option<i32>,
}

pub fn caddy_domains(server: &Server) -> Result<i32, DatabaseError> {
    Ok(server
        .parse_model_extension::<ServerExtension>()?
        .caddy_domains)
}
