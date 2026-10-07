//! Retries Cloudflare deletes that failed while a server was being removed.

use crate::{cloudflare, db};
use shared::State;
use std::time::Duration;

pub async fn run(state: &State) -> Result<(), anyhow::Error> {
    let result = pass(state).await;
    tokio::time::sleep(Duration::from_secs(60)).await;
    result
}

async fn pass(state: &State) -> Result<(), anyhow::Error> {
    let tasks = db::due_cleanup(db::pool(&state.database)).await?;
    for task in tasks {
        let token = match state.database.decrypt_base64(&task.token_encrypted).await {
            Ok(token) => token.to_string(),
            Err(err) => {
                tracing::error!(cleanup = %task.uuid, "caddy: stored cloudflare token could not be decrypted: {err:?}");
                db::postpone_cleanup(
                    db::write_pool(&state.database),
                    task.uuid,
                    task.attempts + 1,
                    "stored cloudflare token could not be decrypted",
                )
                .await?;
                continue;
            }
        };

        match cloudflare::delete_record(&task.zone_id, &token, &task.record_id).await {
            Ok(()) => db::delete_cleanup(db::write_pool(&state.database), task.uuid).await?,
            Err(err) => {
                tracing::warn!(cleanup = %task.uuid, "caddy: dns cleanup retry failed: {err:?}");
                db::postpone_cleanup(
                    db::write_pool(&state.database),
                    task.uuid,
                    task.attempts + 1,
                    &err.to_string(),
                )
                .await?;
            }
        }
    }

    Ok(())
}
