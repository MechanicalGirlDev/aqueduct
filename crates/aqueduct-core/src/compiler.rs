//! グラフコンパイラ。

use std::collections::{BTreeSet, HashMap, VecDeque};

use aqueduct_protocol::{Graph, NodeId, PinValue};

use crate::error::{AqueductError, AqueductResult, ErrorKind};
use crate::graph::{topological_sort, validate_graph};
use crate::node::{NodeEvalResult, NodeEvaluator};
use crate::pin_store::{PinStore, ScopedPinId};
use crate::registry::NodeRegistry;

/// コンパイル済みノード。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompiledNode {
    /// ノード ID。
    pub node_id: NodeId,
    /// 入力ピン順序。
    pub input_pins: Vec<ScopedPinId>,
    /// 出力ピン順序。
    pub output_pins: Vec<ScopedPinId>,
}

/// 非同期ジョブの完了結果。
#[derive(Debug, Clone, PartialEq)]
pub struct AsyncJobResult {
    /// ジョブ識別子。
    pub job_id: u64,
    /// 対象ノード。
    pub node_id: NodeId,
    /// ジョブを発火した tick。
    pub tick_spawned: u64,
    /// 非同期ジョブの出力。
    pub outputs: Vec<PinValue>,
}

type EvaluatorMap = HashMap<NodeId, Box<dyn NodeEvaluator>>;

#[derive(Debug, Clone, PartialEq, Eq)]
struct PendingAsyncJob {
    node_id: NodeId,
    tick_spawned: u64,
    graph_rev: u64,
}

/// コンパイル済みグラフ。
pub struct CompiledGraph {
    graph: Graph,
    eval_order: Vec<NodeId>,
    nodes: HashMap<NodeId, CompiledNode>,
    connections: HashMap<ScopedPinId, ScopedPinId>,
    graph_rev: u64,
    pin_store: PinStore,
    evaluators: EvaluatorMap,
    pending_jobs: HashMap<u64, PendingAsyncJob>,
    completed_jobs: VecDeque<AsyncJobResult>,
}

impl CompiledGraph {
    /// 1 tick 分を評価する。
    ///
    /// # Errors
    /// 評価器が存在しない場合、非同期ジョブ反映失敗、または評価失敗時にエラーを返します。
    pub fn tick(&mut self, tick: u64) -> AqueductResult<()> {
        self.drain_completed_jobs()?;

        let eval_order = self.eval_order.clone();
        for node_id in eval_order {
            let Some(node) = self.nodes.get(&node_id).cloned() else {
                return Err(AqueductError::new(
                    ErrorKind::Runtime,
                    "COMPILED_NODE_PLAN_MISSING",
                    format!("コンパイル済みノード計画が見つかりません: {node_id}"),
                ));
            };

            let inputs = self.collect_inputs(&node);
            let Some(evaluator) = self.evaluators.get_mut(&node_id) else {
                return Err(AqueductError::new(
                    ErrorKind::Runtime,
                    "COMPILED_EVALUATOR_MISSING",
                    format!("評価器が見つかりません: {node_id}"),
                ));
            };

            let result = evaluator.evaluate(&inputs, tick)?;
            self.apply_result(&node, result, tick)?;
        }

        self.pin_store.clear_events();
        Ok(())
    }

    /// 完了済み非同期ジョブを `PinStore` に反映する。
    ///
    /// # Errors
    /// 非同期ジョブ出力の反映に失敗した場合にエラーを返します。
    pub fn drain_completed_jobs(&mut self) -> AqueductResult<()> {
        while let Some(result) = self.completed_jobs.pop_front() {
            let Some(node) = self.nodes.get(&result.node_id).cloned() else {
                continue;
            };
            Self::write_outputs(&node, result.outputs, &mut self.pin_store)?;
        }

        Ok(())
    }

    /// 非同期ジョブ完了結果をキューへ注入する。
    ///
    /// 戻り値 `true` は受理、`false` は世代不一致や未知ジョブとして破棄を表します。
    #[must_use]
    pub fn complete_job(&mut self, result: AsyncJobResult) -> bool {
        let Some(pending) = self.pending_jobs.get(&result.job_id) else {
            return false;
        };

        if pending.graph_rev != self.graph_rev
            || pending.node_id != result.node_id
            || pending.tick_spawned != result.tick_spawned
        {
            return false;
        }

        let _ = self.pending_jobs.remove(&result.job_id);
        self.completed_jobs.push_back(result);
        true
    }

    /// 保留中ジョブ数を返す。
    #[must_use]
    pub fn pending_job_count(&self) -> usize {
        self.pending_jobs.len()
    }

    /// 指定ピンの値を取得する。
    #[must_use]
    pub fn pin_value(&self, pin_id: &ScopedPinId) -> Option<PinValue> {
        self.pin_store.get_value(pin_id).cloned()
    }

    /// 指定ピンのイベント発火状態を取得する。
    #[must_use]
    pub fn is_event_fired(&self, pin_id: &ScopedPinId) -> bool {
        self.pin_store.is_event_fired(pin_id)
    }

    /// ピンストアのスナップショットを取得する。
    #[must_use]
    pub fn pin_store_snapshot(&self) -> PinStore {
        self.pin_store.clone()
    }

    /// 既存ピン値を引き継ぐ。
    pub fn copy_pin_values_from(&mut self, previous: &PinStore, affected_nodes: &BTreeSet<NodeId>) {
        for (pin_id, value) in previous.value_entries() {
            if affected_nodes.contains(&pin_id.node_id) {
                continue;
            }

            if self.pin_store.contains_value_pin(pin_id) {
                self.pin_store.set_value_if_present(pin_id, value.clone());
            }
        }
    }

    /// ノードのプロパティパッチを評価器へ直接適用する。
    ///
    /// # Errors
    /// 対象ノード/評価器が存在しない場合、または評価器のパッチ適用失敗時にエラーを返します。
    pub fn apply_property_patch(
        &mut self,
        node_id: &NodeId,
        key: &str,
        value: &serde_json::Value,
    ) -> AqueductResult<()> {
        let Some(node) = self.graph.nodes.get_mut(node_id) else {
            return Err(AqueductError::new(
                ErrorKind::Runtime,
                "COMPILED_PROPERTY_TARGET_MISSING",
                format!("プロパティ適用先ノードが見つかりません: {node_id}"),
            ));
        };
        let _ = node.properties.insert(key.to_owned(), value.clone());

        let Some(evaluator) = self.evaluators.get_mut(node_id) else {
            return Err(AqueductError::new(
                ErrorKind::Runtime,
                "COMPILED_EVALUATOR_MISSING",
                format!("評価器が見つかりません: {node_id}"),
            ));
        };
        evaluator.apply_property_patch(key, value)
    }

    /// 値ピンへ直接書き込む。
    pub fn set_pin_value(&mut self, pin_id: ScopedPinId, value: PinValue) {
        self.pin_store.set_value(pin_id, value);
    }

    /// グラフリビジョンを返す。
    #[must_use]
    pub const fn graph_rev(&self) -> u64 {
        self.graph_rev
    }

    /// 評価順を返す。
    #[must_use]
    pub fn eval_order(&self) -> &[NodeId] {
        &self.eval_order
    }

    /// 元グラフ定義を返す。
    #[must_use]
    pub fn source_graph(&self) -> &Graph {
        &self.graph
    }

    fn collect_inputs(&self, node: &CompiledNode) -> Vec<PinValue> {
        node.input_pins
            .iter()
            .map(|input_pin| {
                let source_pin = self.connections.get(input_pin).unwrap_or(input_pin);
                if self.pin_store.is_event_fired(source_pin) {
                    return PinValue::Event;
                }

                self.pin_store
                    .get_value(source_pin)
                    .cloned()
                    .unwrap_or(PinValue::None)
            })
            .collect()
    }

    fn apply_result(
        &mut self,
        node: &CompiledNode,
        result: NodeEvalResult,
        tick: u64,
    ) -> AqueductResult<()> {
        match result {
            NodeEvalResult::Ready(outputs) => {
                Self::write_outputs(node, outputs, &mut self.pin_store)
            }
            NodeEvalResult::Spawned { job_id } => {
                self.register_pending_job(node.node_id.clone(), job_id, tick)
            }
        }
    }

    fn register_pending_job(
        &mut self,
        node_id: NodeId,
        job_id: u64,
        tick_spawned: u64,
    ) -> AqueductResult<()> {
        let Some(_node) = self.nodes.get(&node_id) else {
            return Err(AqueductError::new(
                ErrorKind::Runtime,
                "COMPILED_ASYNC_NODE_MISSING",
                format!("非同期ジョブ対象ノードが見つかりません: {node_id}"),
            ));
        };

        if self.pending_jobs.contains_key(&job_id) {
            return Err(AqueductError::new(
                ErrorKind::Runtime,
                "COMPILED_ASYNC_JOB_DUPLICATE",
                format!("重複した job_id を検出しました: {job_id}"),
            ));
        }

        let _ = self.pending_jobs.insert(
            job_id,
            PendingAsyncJob {
                node_id,
                tick_spawned,
                graph_rev: self.graph_rev,
            },
        );
        Ok(())
    }

    fn write_outputs(
        node: &CompiledNode,
        outputs: Vec<PinValue>,
        pin_store: &mut PinStore,
    ) -> AqueductResult<()> {
        if outputs.len() != node.output_pins.len() {
            return Err(AqueductError::new(
                ErrorKind::Runtime,
                "COMPILED_OUTPUT_LENGTH_MISMATCH",
                format!(
                    "ノード {} の出力数が一致しません: expected={}, actual={}",
                    node.node_id,
                    node.output_pins.len(),
                    outputs.len()
                ),
            ));
        }

        for (pin_id, value) in node.output_pins.iter().cloned().zip(outputs.into_iter()) {
            if value == PinValue::Event {
                pin_store.fire_event(pin_id);
            } else {
                pin_store.set_value(pin_id, value);
            }
        }

        Ok(())
    }
}

/// グラフコンパイラ。
pub struct GraphCompiler<'a> {
    registry: &'a NodeRegistry,
}

impl<'a> GraphCompiler<'a> {
    /// コンパイラを作成する。
    #[must_use]
    pub const fn new(registry: &'a NodeRegistry) -> Self {
        Self { registry }
    }

    /// グラフをコンパイルする。
    ///
    /// # Errors
    /// グラフ検証失敗、ノード生成失敗、または接続マップ構築失敗時にエラーを返します。
    pub fn compile(&self, graph: &Graph, graph_rev: u64) -> AqueductResult<CompiledGraph> {
        validate_graph(graph, self.registry)?;

        let eval_order = topological_sort(graph)?;
        let mut evaluators: EvaluatorMap = HashMap::with_capacity(eval_order.len());
        let mut nodes: HashMap<NodeId, CompiledNode> = HashMap::with_capacity(eval_order.len());

        for node_id in &eval_order {
            let Some(node) = graph.nodes.get(node_id) else {
                return Err(AqueductError::new(
                    ErrorKind::Compile,
                    "COMPILER_NODE_INSTANCE_MISSING",
                    format!("ノードインスタンスが見つかりません: {node_id}"),
                ));
            };

            let Some(factory) = self.registry.get(&node.type_name) else {
                let type_name = &node.type_name;
                return Err(AqueductError::new(
                    ErrorKind::Compile,
                    "COMPILER_FACTORY_MISSING",
                    format!("ノード型 {type_name} のファクトリが見つかりません"),
                ));
            };

            let node_def = factory.node_def();
            let evaluator = factory.create(&node.properties)?;

            let compiled_node = CompiledNode {
                node_id: node_id.clone(),
                input_pins: node_def
                    .inputs
                    .iter()
                    .map(|pin| ScopedPinId::new(node_id.clone(), pin.id.clone()))
                    .collect(),
                output_pins: node_def
                    .outputs
                    .iter()
                    .map(|pin| ScopedPinId::new(node_id.clone(), pin.id.clone()))
                    .collect(),
            };

            nodes.insert(node_id.clone(), compiled_node);
            evaluators.insert(node_id.clone(), evaluator);
        }

        let connections = build_connection_map(graph);
        let pin_store = PinStore::from_graph(graph, self.registry)?;

        Ok(CompiledGraph {
            graph: graph.clone(),
            eval_order,
            nodes,
            connections,
            graph_rev,
            pin_store,
            evaluators,
            pending_jobs: HashMap::new(),
            completed_jobs: VecDeque::new(),
        })
    }
}

fn build_connection_map(graph: &Graph) -> HashMap<ScopedPinId, ScopedPinId> {
    let mut connections = HashMap::with_capacity(graph.edges.len());
    for edge in &graph.edges {
        let to_pin = ScopedPinId::new(edge.to_node.clone(), edge.to_pin.clone());
        let from_pin = ScopedPinId::new(edge.from_node.clone(), edge.from_pin.clone());
        let _ = connections.insert(to_pin, from_pin);
    }

    connections
}
