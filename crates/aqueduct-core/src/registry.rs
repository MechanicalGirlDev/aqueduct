//! ノード定義レジストリ。

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use aqueduct_protocol::NodeDef;

use crate::error::{AqueductError, AqueductResult, ErrorKind};
use crate::node::NodeFactory;

/// ノードファクトリ管理。
#[derive(Default)]
pub struct NodeRegistry {
    factories: HashMap<String, Arc<dyn NodeFactory>>,
    defs: HashMap<String, NodeDef>,
}

impl NodeRegistry {
    /// 空のレジストリを作る。
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// ノードファクトリを登録する。
    ///
    /// # Errors
    /// 型名が空、または同名型が既に登録済みの場合にエラーを返します。
    pub fn register(&mut self, factory: Arc<dyn NodeFactory>) -> AqueductResult<()> {
        let node_def = factory.node_def();
        let type_name = node_def.type_name.clone();
        if type_name.trim().is_empty() {
            return Err(AqueductError::new(
                ErrorKind::Registry,
                "REGISTRY_EMPTY_TYPE_NAME",
                "type_name は空文字列を許可しません",
            ));
        }
        if self.factories.contains_key(&type_name) {
            return Err(AqueductError::new(
                ErrorKind::Registry,
                "REGISTRY_DUPLICATE_TYPE",
                format!("既に登録済みのノード型です: {type_name}"),
            ));
        }

        self.defs.insert(type_name.clone(), node_def);
        self.factories.insert(type_name, factory);
        Ok(())
    }

    /// ノードファクトリを取得する。
    #[must_use]
    pub fn get(&self, type_name: &str) -> Option<Arc<dyn NodeFactory>> {
        self.factories.get(type_name).cloned()
    }

    /// ノード定義を取得する。
    #[must_use]
    pub fn node_def(&self, type_name: &str) -> Option<&NodeDef> {
        self.defs.get(type_name)
    }

    /// ノード型が登録済みか確認する。
    #[must_use]
    pub fn contains(&self, type_name: &str) -> bool {
        self.factories.contains_key(type_name)
    }

    /// 登録済みノード定義を列挙する。
    #[must_use]
    pub fn list_node_defs(&self) -> Vec<NodeDef> {
        let mut defs: Vec<NodeDef> = self.defs.values().cloned().collect();
        defs.sort_by(|left, right| left.type_name.cmp(&right.type_name));
        defs
    }

    /// WASM プラグインを読み込む。
    ///
    /// # Errors
    /// Step 1 では未実装のため常にエラーを返します。
    pub fn load_wasm_plugin(&mut self, _path: &Path) -> AqueductResult<()> {
        Err(AqueductError::new(
            ErrorKind::Registry,
            "REGISTRY_WASM_NOT_AVAILABLE",
            "Step 1 では WASM プラグイン読み込みは未実装です",
        ))
    }

    /// ビルトインノードを読み込む。
    pub fn load_builtin_nodes(&mut self) {}
}
