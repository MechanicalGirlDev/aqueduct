#![forbid(unsafe_code)]
#![deny(clippy::all, clippy::pedantic)]

//! Tauri v2 向けの Aqueduct サーバー統合。

use std::sync::{Arc, Mutex};

use aqueduct_core::{GraphCompiler, LiveGraph, NodeRegistry};
use aqueduct_protocol::Graph;
use aqueduct_server::{MessageDispatcher, SessionManager};
use tauri::Manager;
use tokio_util::sync::CancellationToken;
use tracing::error;

/// Tauri コマンドハンドラ。
pub mod commands;
mod push;

/// Tauri managed state として保持する Aqueduct エンジン状態。
pub struct AqueductState {
    /// メッセージディスパッチャ。
    pub dispatcher: Arc<MessageDispatcher>,
    /// この Tauri ウィンドウに対応するセッション ID。
    pub session_id: String,
    /// シャットダウントークン。
    pub shutdown: CancellationToken,
}

/// Tauri アプリケーションに Aqueduct エンジンをセットアップします。
///
/// `tauri::Builder::setup` コールバック内で呼び出してください。
///
/// # Errors
/// セッション登録に失敗した場合にエラーを返します。
pub fn setup_aqueduct(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let mut registry = NodeRegistry::new();
    aqueduct_nodes::register_all(&mut registry)?;
    let registry = Arc::new(registry);

    let compiler = GraphCompiler::new(&registry);
    let compiled = compiler.compile(&Graph::default(), 0)?;
    let live_graph = Arc::new(LiveGraph::new(Arc::new(Mutex::new(compiled))));

    let session_manager = Arc::new(SessionManager::new());
    let dispatcher = Arc::new(MessageDispatcher::new(
        Arc::clone(&live_graph),
        Arc::clone(&registry),
        Arc::clone(&session_manager),
    ));

    let session_id = uuid::Uuid::new_v4().to_string();
    session_manager.add_session(session_id.clone())?;

    let shutdown = CancellationToken::new();

    let state = AqueductState {
        dispatcher: Arc::clone(&dispatcher),
        session_id: session_id.clone(),
        shutdown: shutdown.clone(),
    };

    app.manage(state);

    let app_handle = app.handle().clone();
    tauri::async_runtime::spawn(async move {
        if let Err(err) =
            push::run_tauri_tick_loop(dispatcher, session_id, app_handle, shutdown).await
        {
            error!("Tauri tick ループが異常終了しました: {}", err);
        }
    });

    Ok(())
}
