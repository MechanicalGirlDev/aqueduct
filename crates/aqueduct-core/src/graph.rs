//! グラフ検証と解析アルゴリズム。

use std::collections::{BTreeSet, HashMap, HashSet, VecDeque};

use aqueduct_protocol::{Graph, NodeDef, NodeId, PinDef, PinType};

use crate::error::{AqueductError, AqueductResult, ErrorKind};
use crate::registry::NodeRegistry;

/// グラフ整合性を検証する。
///
/// # Errors
/// ノード型未登録、ピン型不一致、循環検出など整合性違反時にエラーを返します。
pub fn validate_graph(graph: &Graph, registry: &NodeRegistry) -> AqueductResult<()> {
    let node_defs = collect_node_defs(graph, registry)?;
    validate_edges(graph, &node_defs)?;
    let _ = topological_sort(graph)?;
    Ok(())
}

/// トポロジカル順を計算する。
///
/// # Errors
/// エッジが未知ノードを参照している場合、または循環が存在する場合にエラーを返します。
pub fn topological_sort(graph: &Graph) -> AqueductResult<Vec<NodeId>> {
    let mut indegree: HashMap<NodeId, usize> = graph
        .nodes
        .keys()
        .cloned()
        .map(|node_id| (node_id, 0))
        .collect();
    let mut outgoing: HashMap<NodeId, Vec<NodeId>> = HashMap::new();

    for edge in &graph.edges {
        if !indegree.contains_key(&edge.from_node) {
            let edge_id = &edge.id;
            let from_node = &edge.from_node;
            return Err(AqueductError::new(
                ErrorKind::Graph,
                "GRAPH_UNKNOWN_EDGE_SOURCE_NODE",
                format!("edge {edge_id} の from_node {from_node} が存在しません"),
            ));
        }

        let Some(target_indegree) = indegree.get_mut(&edge.to_node) else {
            let edge_id = &edge.id;
            let to_node = &edge.to_node;
            return Err(AqueductError::new(
                ErrorKind::Graph,
                "GRAPH_UNKNOWN_EDGE_TARGET_NODE",
                format!("edge {edge_id} の to_node {to_node} が存在しません"),
            ));
        };
        *target_indegree = target_indegree.saturating_add(1);

        outgoing
            .entry(edge.from_node.clone())
            .or_default()
            .push(edge.to_node.clone());
    }

    for targets in outgoing.values_mut() {
        targets.sort();
    }

    let mut queue: BTreeSet<NodeId> = indegree
        .iter()
        .filter(|(_node_id, degree)| **degree == 0)
        .map(|(node_id, _degree)| node_id.clone())
        .collect();

    let mut ordered = Vec::with_capacity(graph.nodes.len());
    while let Some(node_id) = queue.iter().next().cloned() {
        let _ = queue.remove(&node_id);
        ordered.push(node_id.clone());

        if let Some(targets) = outgoing.get(&node_id) {
            for target in targets {
                let Some(target_indegree) = indegree.get_mut(target) else {
                    return Err(AqueductError::new(
                        ErrorKind::Graph,
                        "GRAPH_INDEGREE_TARGET_MISSING",
                        format!("ノード {target} の入次数更新に失敗しました"),
                    ));
                };

                if *target_indegree == 0 {
                    return Err(AqueductError::new(
                        ErrorKind::Graph,
                        "GRAPH_NEGATIVE_INDEGREE",
                        format!("ノード {target} の入次数が不正です"),
                    ));
                }

                *target_indegree -= 1;
                if *target_indegree == 0 {
                    let _ = queue.insert(target.clone());
                }
            }
        }
    }

    if ordered.len() != graph.nodes.len() {
        return Err(AqueductError::new(
            ErrorKind::Graph,
            "GRAPH_CYCLE_DETECTED",
            "循環を検出しました",
        ));
    }

    Ok(ordered)
}

/// 循環が存在するか検出する。
///
/// # Errors
/// グラフ構造が壊れており循環判定自体ができない場合にエラーを返します。
pub fn detect_cycle(graph: &Graph) -> AqueductResult<bool> {
    match topological_sort(graph) {
        Ok(_order) => Ok(false),
        Err(error) if error.code() == "GRAPH_CYCLE_DETECTED" => Ok(true),
        Err(error) => Err(error),
    }
}

/// 変更ノードから下流ノードを列挙する。
#[must_use]
pub fn downstream_nodes(graph: &Graph, changed_nodes: &BTreeSet<NodeId>) -> BTreeSet<NodeId> {
    let mut adjacency: HashMap<NodeId, Vec<NodeId>> = HashMap::new();
    for edge in &graph.edges {
        adjacency
            .entry(edge.from_node.clone())
            .or_default()
            .push(edge.to_node.clone());
    }

    let mut visited = changed_nodes.clone();
    let mut queue: VecDeque<NodeId> = changed_nodes.iter().cloned().collect();
    while let Some(node_id) = queue.pop_front() {
        if let Some(targets) = adjacency.get(&node_id) {
            for target in targets {
                if visited.insert(target.clone()) {
                    queue.push_back(target.clone());
                }
            }
        }
    }

    visited
}

fn collect_node_defs(
    graph: &Graph,
    registry: &NodeRegistry,
) -> AqueductResult<HashMap<NodeId, NodeDef>> {
    let mut node_defs = HashMap::with_capacity(graph.nodes.len());
    for (node_id, node) in &graph.nodes {
        if node.id != *node_id {
            return Err(AqueductError::new(
                ErrorKind::Graph,
                "GRAPH_NODE_ID_MISMATCH",
                format!(
                    "nodes マップのキー {} と NodeInstance.id {} が一致しません",
                    node_id, node.id
                ),
            ));
        }

        let Some(node_def) = registry.node_def(&node.type_name) else {
            let type_name = &node.type_name;
            return Err(AqueductError::new(
                ErrorKind::Graph,
                "GRAPH_UNKNOWN_NODE_TYPE",
                format!("ノード型 {type_name} は未登録です"),
            ));
        };

        for pin in &node_def.inputs {
            if pin.direction != aqueduct_protocol::Direction::Input {
                return Err(AqueductError::new(
                    ErrorKind::Graph,
                    "GRAPH_INVALID_INPUT_DIRECTION",
                    format!(
                        "ノード型 {} の入力ピン {} の方向が Input ではありません",
                        node.type_name, pin.id
                    ),
                ));
            }
        }

        for pin in &node_def.outputs {
            if pin.direction != aqueduct_protocol::Direction::Output {
                return Err(AqueductError::new(
                    ErrorKind::Graph,
                    "GRAPH_INVALID_OUTPUT_DIRECTION",
                    format!(
                        "ノード型 {} の出力ピン {} の方向が Output ではありません",
                        node.type_name, pin.id
                    ),
                ));
            }
        }

        node_defs.insert(node_id.clone(), node_def.clone());
    }

    Ok(node_defs)
}

fn validate_edges(graph: &Graph, node_defs: &HashMap<NodeId, NodeDef>) -> AqueductResult<()> {
    let mut edge_ids = HashSet::with_capacity(graph.edges.len());
    let mut input_connected = HashSet::with_capacity(graph.edges.len());

    for edge in &graph.edges {
        if !edge_ids.insert(edge.id.clone()) {
            let edge_id = &edge.id;
            return Err(AqueductError::new(
                ErrorKind::Graph,
                "GRAPH_DUPLICATE_EDGE_ID",
                format!("重複した edge id: {edge_id}"),
            ));
        }

        let Some(from_def) = node_defs.get(&edge.from_node) else {
            let edge_id = &edge.id;
            let from_node = &edge.from_node;
            return Err(AqueductError::new(
                ErrorKind::Graph,
                "GRAPH_EDGE_FROM_NODE_MISSING",
                format!("edge {edge_id} の from_node {from_node} が存在しません"),
            ));
        };

        let Some(to_def) = node_defs.get(&edge.to_node) else {
            let edge_id = &edge.id;
            let to_node = &edge.to_node;
            return Err(AqueductError::new(
                ErrorKind::Graph,
                "GRAPH_EDGE_TO_NODE_MISSING",
                format!("edge {edge_id} の to_node {to_node} が存在しません"),
            ));
        };

        let Some(from_pin) = find_pin_def(&from_def.outputs, &edge.from_pin) else {
            let edge_id = &edge.id;
            let from_pin = &edge.from_pin;
            return Err(AqueductError::new(
                ErrorKind::Graph,
                "GRAPH_EDGE_FROM_PIN_MISSING",
                format!("edge {edge_id} の from_pin {from_pin} が存在しません"),
            ));
        };

        let Some(to_pin) = find_pin_def(&to_def.inputs, &edge.to_pin) else {
            let edge_id = &edge.id;
            let to_pin = &edge.to_pin;
            return Err(AqueductError::new(
                ErrorKind::Graph,
                "GRAPH_EDGE_TO_PIN_MISSING",
                format!("edge {edge_id} の to_pin {to_pin} が存在しません"),
            ));
        };

        if !input_connected.insert((edge.to_node.clone(), edge.to_pin.clone())) {
            let to_node = &edge.to_node;
            let to_pin = &edge.to_pin;
            return Err(AqueductError::new(
                ErrorKind::Graph,
                "GRAPH_INPUT_MULTI_CONNECTION",
                format!("入力ピン {to_node}.{to_pin} には複数接続できません"),
            ));
        }

        if !pin_types_compatible(from_pin.pin_type, to_pin.pin_type) {
            return Err(AqueductError::new(
                ErrorKind::Graph,
                "GRAPH_PIN_TYPE_MISMATCH",
                format!(
                    "edge {} の型が不一致です: {:?} -> {:?}",
                    edge.id, from_pin.pin_type, to_pin.pin_type
                ),
            ));
        }
    }

    Ok(())
}

fn find_pin_def<'a>(
    pins: &'a [PinDef],
    target_id: &aqueduct_protocol::PinId,
) -> Option<&'a PinDef> {
    pins.iter().find(|pin| pin.id == *target_id)
}

fn pin_types_compatible(from: PinType, to: PinType) -> bool {
    if from == PinType::Any || to == PinType::Any {
        return true;
    }

    if from == PinType::Event || to == PinType::Event {
        return from == PinType::Event && to == PinType::Event;
    }

    from == to
}
