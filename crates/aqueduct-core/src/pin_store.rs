//! ピン値ストア。

use std::collections::{HashMap, HashSet};

use aqueduct_protocol::{Graph, NodeId, PinId, PinType, PinValue};

use crate::error::{AqueductError, AqueductResult, ErrorKind};
use crate::registry::NodeRegistry;

/// ノードインスタンスで修飾されたピン ID。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ScopedPinId {
    /// 所属ノード。
    pub node_id: NodeId,
    /// ノードローカルのピン ID。
    pub pin_id: PinId,
}

impl ScopedPinId {
    /// 新しい修飾ピン ID を作る。
    #[must_use]
    pub const fn new(node_id: NodeId, pin_id: PinId) -> Self {
        Self { node_id, pin_id }
    }
}

/// 値ピンとイベントピンの状態を保持するストア。
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PinStore {
    values: HashMap<ScopedPinId, PinValue>,
    events: HashSet<ScopedPinId>,
}

impl PinStore {
    /// 空のストアを作る。
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// グラフ定義から初期ストアを構築する。
    ///
    /// # Errors
    /// ノード型が未登録でピン定義を取得できない場合にエラーを返します。
    pub fn from_graph(graph: &Graph, registry: &NodeRegistry) -> AqueductResult<Self> {
        let mut store = Self::new();

        for node in graph.nodes.values() {
            let Some(node_def) = registry.node_def(&node.type_name) else {
                let node_type = &node.type_name;
                return Err(AqueductError::new(
                    ErrorKind::Storage,
                    "PIN_STORE_NODE_DEF_MISSING",
                    format!("ノード型 {node_type} の定義が見つかりません"),
                ));
            };

            for pin in node_def.inputs.iter().chain(&node_def.outputs) {
                if let Some(default_value) = default_pin_value(pin.pin_type) {
                    let scoped_pin = ScopedPinId::new(node.id.clone(), pin.id.clone());
                    let _ = store.values.entry(scoped_pin).or_insert(default_value);
                }
            }
        }

        Ok(store)
    }

    /// 値ピンを読み取る。
    #[must_use]
    pub fn get_value(&self, pin_id: &ScopedPinId) -> Option<&PinValue> {
        self.values.get(pin_id)
    }

    /// 値ピンを保持しているか判定する。
    #[must_use]
    pub fn contains_value_pin(&self, pin_id: &ScopedPinId) -> bool {
        self.values.contains_key(pin_id)
    }

    /// イベントピン発火状態を読み取る。
    #[must_use]
    pub fn is_event_fired(&self, pin_id: &ScopedPinId) -> bool {
        self.events.contains(pin_id)
    }

    /// 値ピンへ書き込む。
    pub fn set_value(&mut self, pin_id: ScopedPinId, value: PinValue) {
        let _ = self.values.insert(pin_id, value);
    }

    /// 値ピンが存在する場合だけ書き込む。
    pub fn set_value_if_present(&mut self, pin_id: &ScopedPinId, value: PinValue) {
        if let Some(current_value) = self.values.get_mut(pin_id) {
            *current_value = value;
        }
    }

    /// イベントピンを発火する。
    pub fn fire_event(&mut self, pin_id: ScopedPinId) {
        let _ = self.events.insert(pin_id);
    }

    /// tick の最後にイベントフラグを消去する。
    pub fn clear_events(&mut self) {
        self.events.clear();
    }

    /// 値ピンのイテレータを返す。
    pub fn value_entries(&self) -> impl Iterator<Item = (&ScopedPinId, &PinValue)> {
        self.values.iter()
    }
}

const fn default_pin_value(pin_type: PinType) -> Option<PinValue> {
    match pin_type {
        PinType::Float => Some(PinValue::Float(0.0)),
        PinType::Int => Some(PinValue::Int(0)),
        PinType::Bool => Some(PinValue::Bool(false)),
        PinType::String => Some(PinValue::String(String::new())),
        PinType::Json => Some(PinValue::Json(serde_json::Value::Null)),
        PinType::Any => Some(PinValue::None),
        PinType::Event => None,
    }
}
