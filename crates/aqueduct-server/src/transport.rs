//! トランスポート抽象。

use aqueduct_core::AqueductResult;
use aqueduct_protocol::{ClientEnvelope, ServerEnvelope};

/// 接続を受け入れるサーバー抽象。
#[async_trait::async_trait]
pub trait TransportServer: Send + Sync {
    /// 新しい `TransportSession` を受け入れる。
    ///
    /// # Errors
    /// 接続受け入れに失敗した場合にエラーを返します。
    async fn accept(&self) -> AqueductResult<Box<dyn TransportSession>>;
}

/// クライアントごとのセッション抽象。
#[async_trait::async_trait]
pub trait TransportSession: Send {
    /// クライアントから `ClientEnvelope` を受信する。
    ///
    /// `None` は切断を表します。
    ///
    /// # Errors
    /// 受信処理に失敗した場合にエラーを返します。
    async fn recv(&mut self) -> AqueductResult<Option<ClientEnvelope>>;

    /// クライアントへ `ServerEnvelope` を送信する。
    ///
    /// # Errors
    /// 送信処理に失敗した場合にエラーを返します。
    async fn send(&self, msg: ServerEnvelope) -> AqueductResult<()>;

    /// セッション ID を返す。
    fn session_id(&self) -> &str;
}
