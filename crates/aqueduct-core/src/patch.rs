//! グラフ差分適用。

use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};

use aqueduct_protocol::{Graph, GraphMutation, NodeId};

use crate::compiler::GraphCompiler;
use crate::error::{AqueductError, AqueductResult, ErrorKind};
use crate::graph::downstream_nodes;
use crate::registry::NodeRegistry;
use crate::runtime::LiveGraph;

/// パッチ適用結果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatchReport {
    /// 影響ノード集合。
    pub affected_nodes: BTreeSet<NodeId>,
    /// 適用後グラフリビジョン。
    pub graph_rev: u64,
}

/// グラフ差分適用器。
pub struct GraphPatcher<'a> {
    compiler: GraphCompiler<'a>,
}

impl<'a> GraphPatcher<'a> {
    /// パッチ適用器を作成する。
    #[must_use]
    pub const fn new(registry: &'a NodeRegistry) -> Self {
        Self {
            compiler: GraphCompiler::new(registry),
        }
    }

    /// `GraphMutation` を適用して `LiveGraph` を差し替える。
    ///
    /// # Errors
    /// パッチ適用失敗、再コンパイル失敗、または状態引き継ぎ失敗時にエラーを返します。
    pub fn patch_live_graph(
        &self,
        live_graph: &LiveGraph,
        mutations: &[GraphMutation],
    ) -> AqueductResult<PatchReport> {
        let current_rev = live_graph.with_graph(|graph| Ok(graph.graph_rev()))?;
        if mutations.is_empty() {
            return Ok(PatchReport {
                affected_nodes: BTreeSet::new(),
                graph_rev: current_rev,
            });
        }

        if is_property_only_batch(mutations) {
            return patch_property_only_batch(live_graph, mutations, current_rev);
        }

        let mut next_graph = live_graph.with_graph(|graph| Ok(graph.source_graph().clone()))?;
        let changed_nodes = apply_mutations(&mut next_graph, mutations)?;
        let affected_nodes = downstream_nodes(&next_graph, &changed_nodes);

        let next_rev = current_rev.saturating_add(1);
        let mut compiled = self.compiler.compile(&next_graph, next_rev)?;
        let previous_store = live_graph.with_graph(|graph| Ok(graph.pin_store_snapshot()))?;
        compiled.copy_pin_values_from(&previous_store, &affected_nodes);

        live_graph.replace(Arc::new(Mutex::new(compiled)));

        Ok(PatchReport {
            affected_nodes,
            graph_rev: next_rev,
        })
    }
}

fn is_property_only_batch(mutations: &[GraphMutation]) -> bool {
    mutations
        .iter()
        .all(|mutation| matches!(mutation, GraphMutation::UpdateProperty { .. }))
}

fn patch_property_only_batch(
    live_graph: &LiveGraph,
    mutations: &[GraphMutation],
    current_rev: u64,
) -> AqueductResult<PatchReport> {
    let changed_nodes = collect_property_changed_nodes(mutations);
    let affected_nodes = live_graph.with_graph_mut(|graph| {
        for mutation in mutations {
            if let GraphMutation::UpdateProperty {
                node_id,
                key,
                value,
            } = mutation
            {
                graph.apply_property_patch(node_id, key, value)?;
            }
        }

        Ok(downstream_nodes(graph.source_graph(), &changed_nodes))
    })?;

    Ok(PatchReport {
        affected_nodes,
        graph_rev: current_rev,
    })
}

fn collect_property_changed_nodes(mutations: &[GraphMutation]) -> BTreeSet<NodeId> {
    let mut changed_nodes = BTreeSet::new();
    for mutation in mutations {
        if let GraphMutation::UpdateProperty { node_id, .. } = mutation {
            let _ = changed_nodes.insert(node_id.clone());
        }
    }
    changed_nodes
}

/// グラフへ差分を適用して変更ノード集合を返す。
///
/// # Errors
/// 追加/削除対象が存在しない場合や ID が衝突した場合にエラーを返します。
pub fn apply_mutations(
    graph: &mut Graph,
    mutations: &[GraphMutation],
) -> AqueductResult<BTreeSet<NodeId>> {
    let mut changed_nodes = BTreeSet::new();

    for mutation in mutations {
        match mutation {
            GraphMutation::AddNode { instance } => {
                if graph.nodes.contains_key(&instance.id) {
                    let instance_id = &instance.id;
                    return Err(AqueductError::new(
                        ErrorKind::Patch,
                        "PATCH_DUPLICATE_NODE_ID",
                        format!("既に存在する node id です: {instance_id}"),
                    ));
                }

                changed_nodes.insert(instance.id.clone());
                let _ = graph.nodes.insert(instance.id.clone(), instance.clone());
            }
            GraphMutation::RemoveNode { id } => {
                if graph.nodes.remove(id).is_none() {
                    return Err(AqueductError::new(
                        ErrorKind::Patch,
                        "PATCH_REMOVE_NODE_NOT_FOUND",
                        format!("削除対象ノードが存在しません: {id}"),
                    ));
                }

                graph
                    .edges
                    .retain(|edge| edge.from_node != *id && edge.to_node != *id);
                let _ = changed_nodes.insert(id.clone());
            }
            GraphMutation::AddEdge { edge } => {
                if graph.edges.iter().any(|current| current.id == edge.id) {
                    let edge_id = &edge.id;
                    return Err(AqueductError::new(
                        ErrorKind::Patch,
                        "PATCH_DUPLICATE_EDGE_ID",
                        format!("既に存在する edge id です: {edge_id}"),
                    ));
                }

                graph.edges.push(edge.clone());
                let _ = changed_nodes.insert(edge.from_node.clone());
                let _ = changed_nodes.insert(edge.to_node.clone());
            }
            GraphMutation::RemoveEdge { id } => {
                let Some(position) = graph.edges.iter().position(|edge| edge.id == *id) else {
                    return Err(AqueductError::new(
                        ErrorKind::Patch,
                        "PATCH_REMOVE_EDGE_NOT_FOUND",
                        format!("削除対象エッジが存在しません: {id}"),
                    ));
                };

                let edge = graph.edges.remove(position);
                let _ = changed_nodes.insert(edge.from_node);
                let _ = changed_nodes.insert(edge.to_node);
            }
            GraphMutation::UpdateProperty {
                node_id,
                key,
                value,
            } => {
                let Some(node) = graph.nodes.get_mut(node_id) else {
                    return Err(AqueductError::new(
                        ErrorKind::Patch,
                        "PATCH_PROPERTY_NODE_NOT_FOUND",
                        format!("更新対象ノードが存在しません: {node_id}"),
                    ));
                };

                let _ = node.properties.insert(key.clone(), value.clone());
                let _ = changed_nodes.insert(node_id.clone());
            }
        }
    }

    Ok(changed_nodes)
}
