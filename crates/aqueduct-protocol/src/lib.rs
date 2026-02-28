#![forbid(unsafe_code)]
#![deny(clippy::all, clippy::pedantic)]

//! Aqueduct の共有プロトコル型定義。

use std::collections::HashMap;
use std::fmt::{Display, Formatter, Result as FmtResult};

use serde::{Deserialize, Serialize};

/// プロトコル互換性確認に使うバージョン文字列。
pub const PROTOCOL_VERSION: &str = "0.1.0";

/// ピン ID。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PinId(pub String);

impl PinId {
    /// 新しい ID を生成する。
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
}

impl From<&str> for PinId {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}

impl From<String> for PinId {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl Display for PinId {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        f.write_str(&self.0)
    }
}

/// ノード ID。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct NodeId(pub String);

impl NodeId {
    /// 新しい ID を生成する。
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
}

impl From<&str> for NodeId {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}

impl From<String> for NodeId {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl Display for NodeId {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        f.write_str(&self.0)
    }
}

/// エッジ ID。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EdgeId(pub String);

impl EdgeId {
    /// 新しい ID を生成する。
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
}

impl From<&str> for EdgeId {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}

impl From<String> for EdgeId {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl Display for EdgeId {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        f.write_str(&self.0)
    }
}

/// ピンで扱うデータ型。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PinType {
    /// 浮動小数。
    Float,
    /// 整数。
    Int,
    /// 真偽値。
    Bool,
    /// 文字列。
    String,
    /// 任意 JSON。
    Json,
    /// イベントパルス。
    Event,
    /// 接続時に解決される動的型。
    Any,
}

/// ピンの値。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum PinValue {
    /// 浮動小数値。
    Float(f64),
    /// 整数値。
    Int(i64),
    /// 真偽値。
    Bool(bool),
    /// 文字列値。
    String(String),
    /// 任意 JSON 値。
    Json(serde_json::Value),
    /// イベント発火。
    Event,
    /// 値なし。
    None,
}

/// ピン方向。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    /// 入力ピン。
    Input,
    /// 出力ピン。
    Output,
}

/// ノードのプロパティ定義。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Property {
    /// プロパティキー。
    pub key: String,
    /// 表示名。
    pub name: String,
    /// 説明。
    #[serde(default)]
    pub description: Option<String>,
    /// デフォルト値。
    #[serde(default)]
    pub default_value: serde_json::Value,
}

/// ピン定義。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PinDef {
    /// ピン ID。
    pub id: PinId,
    /// 表示名。
    pub name: String,
    /// 型。
    pub pin_type: PinType,
    /// 方向。
    pub direction: Direction,
}

/// ノード定義。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NodeDef {
    /// ノード型名。
    pub type_name: String,
    /// 入力ピン定義。
    pub inputs: Vec<PinDef>,
    /// 出力ピン定義。
    pub outputs: Vec<PinDef>,
    /// プロパティ定義。
    #[serde(default)]
    pub properties: Vec<Property>,
}

/// グラフ上のノードインスタンス。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NodeInstance {
    /// ノード ID。
    pub id: NodeId,
    /// ノード型名。
    pub type_name: String,
    /// インスタンス固有プロパティ。
    #[serde(default)]
    pub properties: HashMap<String, serde_json::Value>,
    /// GUI 表示位置。
    pub position: (f32, f32),
}

/// ノード間接続。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Edge {
    /// エッジ ID。
    pub id: EdgeId,
    /// 出力元ノード。
    pub from_node: NodeId,
    /// 出力元ピン。
    pub from_pin: PinId,
    /// 入力先ノード。
    pub to_node: NodeId,
    /// 入力先ピン。
    pub to_pin: PinId,
}

/// グラフ全体定義。
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Graph {
    /// ノード一覧。
    #[serde(default)]
    pub nodes: HashMap<NodeId, NodeInstance>,
    /// エッジ一覧。
    #[serde(default)]
    pub edges: Vec<Edge>,
}

/// グラフへ適用する差分操作。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum GraphMutation {
    /// ノード追加。
    AddNode {
        /// 追加するノードインスタンス。
        instance: NodeInstance,
    },
    /// ノード削除。
    RemoveNode {
        /// 削除対象ノード ID。
        id: NodeId,
    },
    /// エッジ追加。
    AddEdge {
        /// 追加するエッジ。
        edge: Edge,
    },
    /// エッジ削除。
    RemoveEdge {
        /// 削除対象エッジ ID。
        id: EdgeId,
    },
    /// ノードプロパティ更新。
    UpdateProperty {
        /// 対象ノード ID。
        node_id: NodeId,
        /// 更新キー。
        key: String,
        /// 更新値。
        value: serde_json::Value,
    },
}

/// 実行状態。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeState {
    /// 実行中。
    Running,
    /// 停止中。
    Stopped,
    /// エラー状態。
    Error,
}

/// クライアント送信メッセージ。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClientMessage {
    /// 接続時の互換性ハンドシェイク。
    #[serde(rename = "handshake")]
    Handshake {
        /// クライアント側プロトコルバージョン。
        protocol_version: String,
    },
    /// グラフ差分適用。
    #[serde(rename = "graph.mutate")]
    GraphMutate {
        /// 差分一覧。
        mutations: Vec<GraphMutation>,
    },
    /// グラフ全体ロード。
    #[serde(rename = "graph.load")]
    GraphLoad {
        /// 読み込むグラフ。
        graph: Graph,
    },
    /// グラフ保存要求。
    #[serde(rename = "graph.save")]
    GraphSave,
    /// コンパイル要求。
    #[serde(rename = "graph.compile")]
    GraphCompile,
    /// ランタイム開始。
    #[serde(rename = "runtime.start")]
    RuntimeStart,
    /// ランタイム停止。
    #[serde(rename = "runtime.stop")]
    RuntimeStop,
    /// Tick レート変更。
    #[serde(rename = "runtime.set_tick_rate")]
    RuntimeSetTickRate {
        /// 目標 Hz。
        hz: f64,
    },
    /// ノード定義一覧取得。
    #[serde(rename = "registry.list")]
    RegistryList,
    /// ピン購読。
    #[serde(rename = "pin.subscribe")]
    PinSubscribe {
        /// 購読対象ピン ID。
        pin_ids: Vec<PinId>,
    },
    /// ピン購読解除。
    #[serde(rename = "pin.unsubscribe")]
    PinUnsubscribe {
        /// 解除対象ピン ID。
        pin_ids: Vec<PinId>,
    },
}

/// サーバー送信メッセージ。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ServerMessage {
    /// 接続時の互換性ハンドシェイク。
    #[serde(rename = "handshake")]
    Handshake {
        /// サーバー側プロトコルバージョン。
        protocol_version: String,
    },
    /// 購読ピンの最新値。
    #[serde(rename = "pin.values")]
    PinValues {
        /// ピン値マップ。
        values: HashMap<PinId, PinValue>,
    },
    /// ランタイム状態通知。
    #[serde(rename = "runtime.state")]
    RuntimeState {
        /// 現在の実行状態。
        state: RuntimeState,
    },
    /// コンパイル結果。
    #[serde(rename = "graph.compiled")]
    GraphCompiled {
        /// 評価順。
        eval_order: Vec<NodeId>,
        /// 警告一覧。
        warnings: Vec<String>,
    },
    /// エラー通知。
    #[serde(rename = "error")]
    Error {
        /// エラーコード。
        code: String,
        /// メッセージ。
        message: String,
        /// 関連ノード。
        #[serde(skip_serializing_if = "Option::is_none")]
        node_id: Option<NodeId>,
    },
    /// ノード定義一覧。
    #[serde(rename = "registry.nodes")]
    RegistryNodes {
        /// 定義一覧。
        defs: Vec<NodeDef>,
    },
}

/// クライアントメッセージ外枠。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClientEnvelope {
    /// リクエスト識別子。
    pub request_id: u64,
    /// 本文。
    pub body: ClientMessage,
}

/// サーバーメッセージ外枠。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ServerEnvelope {
    /// 対応するリクエスト識別子。
    pub request_id: Option<u64>,
    /// 本文。
    pub body: ServerMessage,
    /// サーバー側グラフリビジョン。
    pub graph_rev: u64,
}
