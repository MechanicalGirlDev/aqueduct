#![forbid(unsafe_code)]
#![deny(clippy::all, clippy::pedantic)]

//! Aqueduct のサーバー抽象と実行ループ。

/// メッセージディスパッチャ。
pub mod dispatcher;
/// サーバーループ。
pub mod server_loop;
/// セッション状態管理。
pub mod session;
/// トランスポート抽象。
pub mod transport;

pub use dispatcher::MessageDispatcher;
pub use server_loop::{run_server, run_tick_and_collect_diffs, TickResult};
pub use session::{SessionManager, SessionPinDiff, SessionState};
pub use transport::{TransportServer, TransportSession};
