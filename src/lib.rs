use indexmap::IndexMap;
use shared::{
    extensions::{
        background_tasks::BackgroundTaskBuilder, settings::ExtensionSettingsDeserializer,
        Extension, ExtensionPermissionsBuilder, ExtensionRouteBuilder,
    },
    models::{
        server::{ApiServerFeatureLimits, Server},
        BaseModel, CreatableModel, DeletableModel, ListenerPriority, UpdatableModel,
    },
    permissions::PermissionGroup,
    Extendible, State,
};
use std::sync::Arc;

mod cleanup;
mod cloudflare;
mod db;
mod domain;
mod labels;
mod model;
mod routes;
mod service;
mod settings;

#[derive(Default)]
pub struct ExtensionStruct;

#[async_trait::async_trait]
impl Extension for ExtensionStruct {
    async fn initialize(&mut self, _state: State) {
        Server::register_model_extension(model::ServerExtension);

        Server::register_create_handler(
            ListenerPriority::Normal,
            |options, query_builder, state, _transaction| {
                Box::pin(async move {
                    let limit = match options
                        .feature_limits
                        .parse_extended::<model::ExtendedApiServerFeatureLimits>()
                        .ok()
                        .and_then(|extended| extended.caddy_domains)
                    {
                        Some(limit) => limit,
                        None => settings::load(state)
                            .await
                            .map(|settings| settings.default_limit)
                            .unwrap_or(1),
                    };
                    query_builder.set("caddy_domains", limit.max(0));
                    Ok(())
                })
            },
        );

        Server::register_update_handler(
            ListenerPriority::Normal,
            |_server, options, query_builder, _state, _transaction| {
                Box::pin(async move {
                    if let Some(feature_limits) = &options.feature_limits
                        && let Ok(extended) =
                            feature_limits.parse_extended::<model::ExtendedApiServerFeatureLimits>()
                        && let Some(value) = extended.caddy_domains
                    {
                        query_builder.set("caddy_domains", Some(value.max(0)));
                    }
                    Ok(())
                })
            },
        );

        // Rows cascade with the server. DNS records are removed here, and a
        // failed Cloudflare delete is queued on this same transaction.
        Server::register_delete_handler(
            ListenerPriority::Normal,
            |server, _options, state, transaction| {
                Box::pin(async move {
                    service::on_server_deleted(state, server.uuid, transaction).await
                })
            },
        );

        ApiServerFeatureLimits::extend_validated(
            |server, _state| {
                Box::pin(
                    async move { Ok(server.parse_model_extension::<model::ServerExtension>()?) },
                )
            },
            |_limits, extension, _state| model::ExtendedApiServerFeatureLimits {
                caddy_domains: Some(extension.caddy_domains),
            },
        );
    }

    async fn initialize_router(
        &mut self,
        state: State,
        builder: ExtensionRouteBuilder,
    ) -> ExtensionRouteBuilder {
        builder
            .add_admin_api_router(|router| {
                router.nest("/extensions/dev.minerpl.caddyproxy", routes::admin::router(&state))
            })
            .add_client_server_api_router(|router| {
                router.nest("/caddy", routes::server::router(&state))
            })
    }

    async fn initialize_background_tasks(
        &mut self,
        _state: State,
        builder: BackgroundTaskBuilder,
    ) -> BackgroundTaskBuilder {
        builder
            .add_task("dev-minerpl-caddyproxy-dns-cleanup", |state| async move {
                cleanup::run(&state).await
            })
            .await;
        builder
    }

    async fn initialize_permissions(
        &mut self,
        _state: State,
        builder: ExtensionPermissionsBuilder,
    ) -> ExtensionPermissionsBuilder {
        builder
            .add_server_permission_group(
                "caddy",
                PermissionGroup {
                    description: "Permissions that control Caddy domains for this server.",
                    permissions: IndexMap::from([
                        (
                            "read",
                            "Allows viewing the server's Caddy domains, DNS targets and container labels.",
                        ),
                        (
                            "create",
                            "Allows adding a subdomain or a custom domain to this server.",
                        ),
                        ("update", "Allows changing the container port of a Caddy domain."),
                        (
                            "delete",
                            "Allows removing a Caddy domain, including its Cloudflare DNS record.",
                        ),
                    ]),
                },
            )
            .add_admin_permission_group(
                "caddy",
                PermissionGroup {
                    description: "Permissions that control the Caddy extension.",
                    permissions: IndexMap::from([
                        (
                            "read",
                            "Allows viewing allowed domains, Cloudflare status and every server domain.",
                        ),
                        (
                            "manage",
                            "Allows changing allowed domains, public DNS records, Cloudflare tokens and deleting domains.",
                        ),
                    ]),
                },
            )
    }

    async fn settings_deserializer(&self, _state: State) -> ExtensionSettingsDeserializer {
        Arc::new(settings::ExtensionSettingsDataDeserializer)
    }
}
