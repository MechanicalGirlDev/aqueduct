//! メッセージディスパッチャ。

use std::collections::BTreeSet;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Arc, Mutex, RwLock};

use aqueduct_core::{
    AqueductError, AqueductResult, ErrorKind, GraphCompiler, GraphPatcher, LiveGraph, NodeRegistry,
};
use aqueduct_protocol::{
    ClientEnvelope, ClientMessage, Graph, RuntimeState, ServerEnvelope, ServerMessage,
    PROTOCOL_VERSION,
};

use crate::session::SessionManager;

const RUNTIME_STOPPED: u8 = 0;
const RUNTIME_RUNNING: u8 = 1;
const RUNTIME_ERROR: u8 = 2;
const DEFAULT_TICK_RATE_HZ: f64 = 60.0;

/// `ClientEnvelope` を処理して `ServerEnvelope` を返すディスパッチャ。
pub struct MessageDispatcher {
    live_graph: Arc<LiveGraph>,
    registry: Arc<NodeRegistry>,
    session_manager: Arc<SessionManager>,
    runtime_state: AtomicU8,
    tick_rate_hz: RwLock<f64>,
}

impl MessageDispatcher {
    /// 新しい `MessageDispatcher` を作成します。
    #[must_use]
    pub fn new(
        live_graph: Arc<LiveGraph>,
        registry: Arc<NodeRegistry>,
        session_manager: Arc<SessionManager>,
    ) -> Self {
        Self {
            live_graph,
            registry,
            session_manager,
            runtime_state: AtomicU8::new(RUNTIME_STOPPED),
            tick_rate_hz: RwLock::new(DEFAULT_TICK_RATE_HZ),
        }
    }

    /// 1 メッセージを処理します。
    ///
    /// 処理失敗は `ServerMessage::Error` に変換して返します。
    ///
    /// # Errors
    /// 応答 `graph_rev` の取得に失敗した場合にエラーを返します。
    pub fn dispatch(
        &self,
        session_id: &str,
        envelope: ClientEnvelope,
    ) -> AqueductResult<Option<ServerEnvelope>> {
        let request_id = envelope.request_id;
        let body = match self.handle_message(session_id, envelope.body) {
            Ok(result) => result,
            Err(error) => Some(server_error_message(&error)),
        };

        let Some(body) = body else {
            return Ok(None);
        };

        let graph_rev = self.current_graph_rev()?;
        Ok(Some(ServerEnvelope {
            request_id: Some(request_id),
            body,
            graph_rev,
        }))
    }

    /// 現在の `LiveGraph` を返します。
    #[must_use]
    pub fn live_graph(&self) -> Arc<LiveGraph> {
        Arc::clone(&self.live_graph)
    }

    /// `SessionManager` を返します。
    #[must_use]
    pub fn session_manager(&self) -> Arc<SessionManager> {
        Arc::clone(&self.session_manager)
    }

    /// ランタイムが実行中かどうかを返します。
    #[must_use]
    pub fn is_runtime_running(&self) -> bool {
        self.runtime_state.load(Ordering::SeqCst) == RUNTIME_RUNNING
    }

    /// ランタイムをエラー状態へ遷移させます。
    pub fn mark_runtime_error(&self) {
        self.runtime_state.store(RUNTIME_ERROR, Ordering::SeqCst);
    }

    /// 現在の tick レートを返します。
    ///
    /// # Errors
    /// 内部ロックに失敗した場合にエラーを返します。
    pub fn tick_rate_hz(&self) -> AqueductResult<f64> {
        let guard = self
            .tick_rate_hz
            .read()
            .map_err(|_error| tick_rate_lock_error("DISPATCHER_TICK_RATE_READ_LOCK_POISONED"))?;
        Ok(*guard)
    }

    /// 現在の `graph_rev` を返します。
    ///
    /// # Errors
    /// `LiveGraph` の読み取りに失敗した場合にエラーを返します。
    pub fn current_graph_rev(&self) -> AqueductResult<u64> {
        self.live_graph.with_graph(|graph| Ok(graph.graph_rev()))
    }

    fn handle_message(
        &self,
        session_id: &str,
        message: ClientMessage,
    ) -> AqueductResult<Option<ServerMessage>> {
        match message {
            ClientMessage::Handshake { protocol_version } => {
                if protocol_version != PROTOCOL_VERSION {
                    return Err(AqueductError::new(
                        ErrorKind::Server,
                        "SERVER_PROTOCOL_VERSION_MISMATCH",
                        format!(
                            "プロトコルバージョンが一致しません: client={protocol_version}, server={PROTOCOL_VERSION}"
                        ),
                    ));
                }

                Ok(Some(ServerMessage::Handshake {
                    protocol_version: PROTOCOL_VERSION.to_owned(),
                }))
            }
            ClientMessage::GraphMutate { mutations } => {
                let patcher = GraphPatcher::new(self.registry.as_ref());
                let _report = patcher.patch_live_graph(self.live_graph.as_ref(), &mutations)?;
                let eval_order = self
                    .live_graph
                    .with_graph(|graph| Ok(graph.eval_order().to_vec()))?;

                Ok(Some(ServerMessage::GraphCompiled {
                    eval_order,
                    warnings: Vec::new(),
                }))
            }
            ClientMessage::GraphLoad { graph } => {
                let eval_order = self.compile_and_replace_graph(&graph)?;
                Ok(Some(ServerMessage::GraphCompiled {
                    eval_order,
                    warnings: Vec::new(),
                }))
            }
            ClientMessage::GraphSave => Err(AqueductError::new(
                ErrorKind::Server,
                "SERVER_GRAPH_SAVE_UNSUPPORTED",
                "`graph.save` は未サポートです",
            )),
            ClientMessage::GraphCompile => {
                let graph = self
                    .live_graph
                    .with_graph(|compiled| Ok(compiled.source_graph().clone()))?;
                let eval_order = self.compile_and_replace_graph(&graph)?;
                Ok(Some(ServerMessage::GraphCompiled {
                    eval_order,
                    warnings: Vec::new(),
                }))
            }
            ClientMessage::RuntimeStart => {
                self.runtime_state.store(RUNTIME_RUNNING, Ordering::SeqCst);
                Ok(Some(ServerMessage::RuntimeState {
                    state: RuntimeState::Running,
                }))
            }
            ClientMessage::RuntimeStop => {
                self.runtime_state.store(RUNTIME_STOPPED, Ordering::SeqCst);
                Ok(Some(ServerMessage::RuntimeState {
                    state: RuntimeState::Stopped,
                }))
            }
            ClientMessage::RuntimeSetTickRate { hz } => {
                if !hz.is_finite() || hz <= 0.0 {
                    return Err(AqueductError::new(
                        ErrorKind::Server,
                        "SERVER_INVALID_TICK_RATE",
                        format!("tick レートが不正です: {hz}"),
                    ));
                }

                let mut guard = self.tick_rate_hz.write().map_err(|_error| {
                    tick_rate_lock_error("DISPATCHER_TICK_RATE_WRITE_LOCK_POISONED")
                })?;
                *guard = hz;

                Ok(Some(ServerMessage::RuntimeState {
                    state: self.runtime_state_message(),
                }))
            }
            ClientMessage::RegistryList => Ok(Some(ServerMessage::RegistryNodes {
                defs: self.registry.list_node_defs(),
            })),
            ClientMessage::PinSubscribe { pin_ids } => {
                self.session_manager.subscribe_pins(session_id, &pin_ids)?;
                Ok(None)
            }
            ClientMessage::PinUnsubscribe { pin_ids } => {
                self.session_manager
                    .unsubscribe_pins(session_id, &pin_ids)?;
                Ok(None)
            }
        }
    }

    fn compile_and_replace_graph(
        &self,
        graph: &Graph,
    ) -> AqueductResult<Vec<aqueduct_protocol::NodeId>> {
        let current_rev = self.current_graph_rev()?;
        let next_rev = current_rev.saturating_add(1);
        let graph_compiler = GraphCompiler::new(self.registry.as_ref());
        let mut compiled_graph = graph_compiler.compile(graph, next_rev)?;

        let previous_store = self
            .live_graph
            .with_graph(|existing| Ok(existing.pin_store_snapshot()))?;
        compiled_graph.copy_pin_values_from(&previous_store, &BTreeSet::new());
        let eval_order = compiled_graph.eval_order().to_vec();

        self.live_graph
            .replace(Arc::new(Mutex::new(compiled_graph)));
        Ok(eval_order)
    }

    fn runtime_state_message(&self) -> RuntimeState {
        match self.runtime_state.load(Ordering::SeqCst) {
            RUNTIME_RUNNING => RuntimeState::Running,
            RUNTIME_ERROR => RuntimeState::Error,
            _ => RuntimeState::Stopped,
        }
    }
}

fn server_error_message(error: &AqueductError) -> ServerMessage {
    ServerMessage::Error {
        code: error.code().to_owned(),
        message: error.message().to_owned(),
        node_id: None,
    }
}

fn tick_rate_lock_error(code: &'static str) -> AqueductError {
    AqueductError::new(
        ErrorKind::Server,
        code,
        "MessageDispatcher の tick レートロックが壊れています",
    )
}
