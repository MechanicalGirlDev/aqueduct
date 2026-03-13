//! Tauri コマンドハンドラ。

use aqueduct_protocol::{ClientEnvelope, ServerEnvelope};

use crate::AqueductState;

/// クライアントメッセージをディスパッチして応答を返します。
#[tauri::command]
pub async fn aqueduct_dispatch(
    envelope: ClientEnvelope,
    state: tauri::State<'_, AqueductState>,
) -> Result<Option<ServerEnvelope>, String> {
    state
        .dispatcher
        .dispatch(&state.session_id, envelope)
        .map_err(|e| e.to_string())
}
