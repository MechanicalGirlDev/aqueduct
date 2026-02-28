//! フレームワーク固有エラー型。

use std::error::Error;
use std::fmt::{Display, Formatter, Result as FmtResult};

/// エラー種別。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    /// グラフ定義エラー。
    Graph,
    /// コンパイルエラー。
    Compile,
    /// ノード関連エラー。
    Node,
    /// レジストリエラー。
    Registry,
    /// ランタイムエラー。
    Runtime,
    /// パッチ適用エラー。
    Patch,
    /// ストレージエラー。
    Storage,
    /// サーバー層エラー。
    Server,
}

/// Aqueduct の統一エラー。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AqueductError {
    kind: ErrorKind,
    code: &'static str,
    message: String,
}

impl AqueductError {
    /// 新しいエラーを作成する。
    #[must_use]
    pub fn new(kind: ErrorKind, code: &'static str, message: impl Into<String>) -> Self {
        Self {
            kind,
            code,
            message: message.into(),
        }
    }

    /// 種別を返す。
    #[must_use]
    pub const fn kind(&self) -> ErrorKind {
        self.kind
    }

    /// エラーコードを返す。
    #[must_use]
    pub const fn code(&self) -> &'static str {
        self.code
    }

    /// メッセージを返す。
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl Display for AqueductError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl Error for AqueductError {}

/// Aqueduct の Result 型。
pub type AqueductResult<T> = Result<T, AqueductError>;
