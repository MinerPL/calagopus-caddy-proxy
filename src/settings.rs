use serde::{Deserialize, Serialize};
use shared::extensions::settings::{
    ExtensionSettings, SettingsDeserializeExt, SettingsDeserializer, SettingsSerializeExt,
    SettingsSerializer,
};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(ToSchema, Serialize, Deserialize, Clone, Debug)]
pub struct AllowedDomain {
    pub uuid: Uuid,
    /// Apex users may create names under, such as `example.com`.
    pub domain: String,
    /// `A` or `CNAME`.
    pub record_type: String,
    /// IPv4 address or hostname the subdomain should resolve to.
    pub record_content: String,
    pub cloudflare_enabled: bool,
    pub cloudflare_zone_id: String,
    pub cloudflare_api_token: String,
    pub cloudflare_proxied: bool,
}

#[derive(ToSchema, Serialize, Deserialize, Clone, Debug)]
pub struct ExtensionSettingsData {
    pub allow_custom_domains: bool,
    pub default_limit: i32,
    pub domains: Vec<AllowedDomain>,
}

impl Default for ExtensionSettingsData {
    fn default() -> Self {
        Self {
            allow_custom_domains: true,
            default_limit: 1,
            domains: Vec::new(),
        }
    }
}

impl ExtensionSettingsData {
    pub fn domain(&self, uuid: Uuid) -> Option<&AllowedDomain> {
        self.domains.iter().find(|domain| domain.uuid == uuid)
    }
}

#[async_trait::async_trait]
impl SettingsSerializeExt for ExtensionSettingsData {
    async fn serialize(
        &self,
        serializer: SettingsSerializer,
    ) -> Result<SettingsSerializer, anyhow::Error> {
        Ok(serializer
            .write_serde_encrypted_setting("config", self)
            .await?)
    }
}

pub struct ExtensionSettingsDataDeserializer;

#[async_trait::async_trait]
impl SettingsDeserializeExt for ExtensionSettingsDataDeserializer {
    async fn deserialize_boxed(
        &self,
        deserializer: SettingsDeserializer<'_>,
    ) -> Result<ExtensionSettings, anyhow::Error> {
        Ok(Box::new(
            deserializer
                .read_serde_encrypted_setting::<ExtensionSettingsData>("config")
                .await?
                .unwrap_or_default(),
        ))
    }
}

pub async fn load(state: &shared::State) -> Result<ExtensionSettingsData, anyhow::Error> {
    let settings = state.settings.get().await?;
    let extension: &ExtensionSettingsData = settings.find_extension_settings()?;
    Ok(extension.clone())
}
