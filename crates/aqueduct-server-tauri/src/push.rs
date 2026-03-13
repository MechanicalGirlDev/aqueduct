//! Tauri event 経由のサーバー push ループ。

use std::sync::Arc;
use std::time::Duration;

use aqueduct_core::{AqueductResult, TickDriver};
use aqueduct_protocol::{ServerEnvelope, ServerMessage};
use aqueduct_server::{run_tick_and_collect_diffs, MessageDispatcher};
use tauri::{AppHandle, Emitter};
use tokio_util::sync::CancellationToken;
use tracing::error;

const DEFAULT_TICK_INTERVAL: Duration = Duration::from_millis(16);

/// Tauri の event システムを使ってピン値差分をフロントエンドへ push するループ。
pub(crate) async fn run_tauri_tick_loop(
    dispatcher: Arc<MessageDispatcher>,
    session_id: String,
    app_handle: AppHandle,
    shutdown: CancellationToken,
) -> AqueductResult<()> {
    let mut tick_driver = TickDriver::new(dispatcher.live_graph());
    let mut interval = tokio::time::interval(DEFAULT_TICK_INTERVAL);

    loop {
        tokio::select! {
            () = shutdown.cancelled() => break,
            _ = interval.tick() => {
                let result = run_tick_and_collect_diffs(&mut tick_driver, &dispatcher)?;
                let Some(tick_result) = result else {
                    continue;
                };

                for diff in tick_result.pin_diffs {
                    if diff.session_id != session_id {
                        continue;
                    }

                    let envelope = ServerEnvelope {
                        request_id: None,
                        body: ServerMessage::PinValues {
                            values: diff.values,
                        },
                        graph_rev: tick_result.graph_rev,
                    };

                    if let Err(err) = app_handle.emit("aqueduct://server-message", &envelope) {
                        error!("Tauri event emit に失敗しました: {}", err);
                    }
                }
            }
        }
    }

    Ok(())
}
