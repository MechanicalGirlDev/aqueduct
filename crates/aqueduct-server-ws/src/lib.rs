#![forbid(unsafe_code)]
#![deny(clippy::all, clippy::pedantic)]

//! `axum` WebSocket ベースの `TransportServer` 実装。

use std::net::SocketAddr;

use aqueduct_core::{AqueductError, AqueductResult, ErrorKind};
use aqueduct_protocol::{ClientEnvelope, ServerEnvelope};
use aqueduct_server::{TransportServer, TransportSession};
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Router;
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use tokio::sync::{mpsc, Mutex};
use tower_http::cors::{Any, CorsLayer};
use tracing::{error, warn};
use uuid::Uuid;

/// `axum` WebSocket トランスポートサーバー。
pub struct AxumWsServer {
    bind_addr: SocketAddr,
    accept_rx: Mutex<mpsc::Receiver<AxumWsSession>>,
}

impl AxumWsServer {
    /// 指定アドレスへバインドして WebSocket サーバーを起動します。
    ///
    /// # Errors
    /// ソケットバインドに失敗した場合にエラーを返します。
    pub async fn bind(bind_addr: SocketAddr) -> AqueductResult<Self> {
        let (accept_tx, accept_rx) = mpsc::channel(128);
        let state = AcceptState { accept_tx };
        let cors = CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any);

        let router = Router::new()
            .route("/ws", get(ws_upgrade_handler))
            .layer(cors)
            .with_state(state);

        let listener = tokio::net::TcpListener::bind(bind_addr)
            .await
            .map_err(|error| {
                AqueductError::new(
                    ErrorKind::Server,
                    "AXUM_WS_BIND_FAILED",
                    format!("WebSocket リスナーの bind に失敗しました: {error}"),
                )
            })?;

        tokio::spawn(async move {
            if let Err(error) = axum::serve(listener, router).await {
                error!("WebSocket サーバーが異常終了しました: {error}");
            }
        });

        Ok(Self {
            bind_addr,
            accept_rx: Mutex::new(accept_rx),
        })
    }

    /// bind 先アドレスを返します。
    #[must_use]
    pub const fn bind_addr(&self) -> SocketAddr {
        self.bind_addr
    }
}

#[async_trait::async_trait]
impl TransportServer for AxumWsServer {
    async fn accept(&self) -> AqueductResult<Box<dyn TransportSession>> {
        let mut guard = self.accept_rx.lock().await;
        let Some(session) = guard.recv().await else {
            return Err(AqueductError::new(
                ErrorKind::Server,
                "AXUM_WS_ACCEPT_CHANNEL_CLOSED",
                "WebSocket accept チャネルが閉じています",
            ));
        };

        Ok(Box::new(session))
    }
}

/// `axum` WebSocket セッション。
pub struct AxumWsSession {
    session_id: String,
    sender: Mutex<SplitSink<WebSocket, Message>>,
    receiver: SplitStream<WebSocket>,
}

impl AxumWsSession {
    /// `WebSocket` から `AxumWsSession` を作成します。
    #[must_use]
    pub fn new(socket: WebSocket) -> Self {
        let (sender, receiver) = socket.split();
        Self {
            session_id: Uuid::new_v4().to_string(),
            sender: Mutex::new(sender),
            receiver,
        }
    }
}

#[async_trait::async_trait]
impl TransportSession for AxumWsSession {
    async fn recv(&mut self) -> AqueductResult<Option<ClientEnvelope>> {
        loop {
            let next = self.receiver.next().await;
            let Some(next) = next else {
                return Ok(None);
            };

            match next {
                Ok(Message::Text(text)) => {
                    let envelope =
                        serde_json::from_str::<ClientEnvelope>(text.as_str()).map_err(|error| {
                            AqueductError::new(
                                ErrorKind::Server,
                                "AXUM_WS_MESSAGE_DESERIALIZE_FAILED",
                                format!("ClientEnvelope のデシリアライズに失敗しました: {error}"),
                            )
                        })?;
                    return Ok(Some(envelope));
                }
                Ok(Message::Close(_frame)) => return Ok(None),
                Ok(Message::Ping(_) | Message::Pong(_)) => {}
                Ok(Message::Binary(_payload)) => {
                    return Err(AqueductError::new(
                        ErrorKind::Server,
                        "AXUM_WS_BINARY_MESSAGE_UNSUPPORTED",
                        "Binary メッセージは未サポートです",
                    ));
                }
                Err(error) => {
                    return Err(AqueductError::new(
                        ErrorKind::Server,
                        "AXUM_WS_RECV_FAILED",
                        format!("WebSocket 受信に失敗しました: {error}"),
                    ));
                }
            }
        }
    }

    async fn send(&self, msg: ServerEnvelope) -> AqueductResult<()> {
        let text = serde_json::to_string(&msg).map_err(|error| {
            AqueductError::new(
                ErrorKind::Server,
                "AXUM_WS_MESSAGE_SERIALIZE_FAILED",
                format!("ServerEnvelope のシリアライズに失敗しました: {error}"),
            )
        })?;
        let mut sender = self.sender.lock().await;
        sender.send(Message::Text(text)).await.map_err(|error| {
            AqueductError::new(
                ErrorKind::Server,
                "AXUM_WS_SEND_FAILED",
                format!("WebSocket 送信に失敗しました: {error}"),
            )
        })
    }

    fn session_id(&self) -> &str {
        &self.session_id
    }
}

#[derive(Clone)]
struct AcceptState {
    accept_tx: mpsc::Sender<AxumWsSession>,
}

async fn ws_upgrade_handler(
    ws: WebSocketUpgrade,
    State(state): State<AcceptState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| async move {
        let session = AxumWsSession::new(socket);
        if state.accept_tx.send(session).await.is_err() {
            warn!("accept 待ち受け側が停止しているためセッションを破棄しました");
        }
    })
}
