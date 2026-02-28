//! ノード生成と評価の抽象。

use std::collections::HashMap;

use aqueduct_protocol::{NodeDef, PinValue};

use crate::error::AqueductResult;

/// ノード評価結果。
#[derive(Debug, Clone, PartialEq)]
#[must_use]
pub enum NodeEvalResult {
    /// 同期的に出力が確定した。
    Ready(Vec<PinValue>),
    /// 非同期ジョブを発火した。
    Spawned {
        /// ジョブ識別子。
        job_id: u64,
    },
}

/// ノード評価器。
pub trait NodeEvaluator: Send {
    /// 入力値を受けて 1 tick 分評価する。
    ///
    /// # Errors
    /// 入力不正や内部評価失敗時にエラーを返します。
    fn evaluate(&mut self, inputs: &[PinValue], tick: u64) -> AqueductResult<NodeEvalResult>;

    /// プロパティ差分を適用する。
    ///
    /// # Errors
    /// パッチ値不正や更新失敗時にエラーを返します。
    fn apply_property_patch(&mut self, key: &str, value: &serde_json::Value) -> AqueductResult<()>;
}

/// ノード評価器を生成するファクトリ。
pub trait NodeFactory: Send + Sync {
    /// ノード定義を返す。
    fn node_def(&self) -> NodeDef;

    /// ノードインスタンスのプロパティから評価器を生成する。
    ///
    /// # Errors
    /// 必須プロパティ不足や設定値不正時にエラーを返します。
    fn create(
        &self,
        properties: &HashMap<String, serde_json::Value>,
    ) -> AqueductResult<Box<dyn NodeEvaluator>>;
}
