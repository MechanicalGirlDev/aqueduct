#![allow(clippy::too_many_lines)]

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use aqueduct_core::{
    apply_mutations, detect_cycle, topological_sort, AsyncJobResult, GraphCompiler, GraphPatcher,
    LiveGraph, NodeEvalResult, NodeEvaluator, NodeFactory, NodeRegistry, PinStore, ScopedPinId,
    TickDriver,
};
use aqueduct_core::{AqueductError, AqueductResult, ErrorKind};
use aqueduct_protocol::{
    Direction, Edge, EdgeId, Graph, GraphMutation, NodeDef, NodeId, NodeInstance, PinDef, PinId,
    PinType, PinValue,
};
use serde_json::json;

struct ConstFactory {
    type_name: &'static str,
    output_pin: PinId,
}

impl ConstFactory {
    fn new(type_name: &'static str, output_pin: PinId) -> Self {
        Self {
            type_name,
            output_pin,
        }
    }
}

impl NodeFactory for ConstFactory {
    fn node_def(&self) -> NodeDef {
        NodeDef {
            type_name: self.type_name.to_owned(),
            inputs: Vec::new(),
            outputs: vec![PinDef {
                id: self.output_pin.clone(),
                name: "out".to_owned(),
                pin_type: PinType::Float,
                direction: Direction::Output,
            }],
            properties: Vec::new(),
        }
    }

    fn create(
        &self,
        properties: &HashMap<String, serde_json::Value>,
    ) -> AqueductResult<Box<dyn NodeEvaluator>> {
        let Some(value) = properties.get("value") else {
            return Err(AqueductError::new(
                ErrorKind::Node,
                "TEST_CONST_PROPERTY_MISSING",
                "const ノードは value プロパティを必要とします",
            ));
        };

        let Some(value) = value.as_f64() else {
            return Err(AqueductError::new(
                ErrorKind::Node,
                "TEST_CONST_PROPERTY_INVALID",
                "value は f64 である必要があります",
            ));
        };

        Ok(Box::new(ConstEvaluator { value }))
    }
}

struct ConstEvaluator {
    value: f64,
}

impl NodeEvaluator for ConstEvaluator {
    fn evaluate(&mut self, _inputs: &[PinValue], _tick: u64) -> AqueductResult<NodeEvalResult> {
        Ok(NodeEvalResult::Ready(vec![PinValue::Float(self.value)]))
    }

    fn apply_property_patch(&mut self, key: &str, value: &serde_json::Value) -> AqueductResult<()> {
        if key != "value" {
            return Ok(());
        }

        let Some(updated) = value.as_f64() else {
            return Err(AqueductError::new(
                ErrorKind::Node,
                "TEST_CONST_PATCH_INVALID",
                "value は f64 である必要があります",
            ));
        };

        self.value = updated;
        Ok(())
    }
}

struct AddFactory;

impl NodeFactory for AddFactory {
    fn node_def(&self) -> NodeDef {
        NodeDef {
            type_name: "math.add".to_owned(),
            inputs: vec![
                PinDef {
                    id: PinId::from("add_in_a"),
                    name: "a".to_owned(),
                    pin_type: PinType::Float,
                    direction: Direction::Input,
                },
                PinDef {
                    id: PinId::from("add_in_b"),
                    name: "b".to_owned(),
                    pin_type: PinType::Float,
                    direction: Direction::Input,
                },
            ],
            outputs: vec![PinDef {
                id: PinId::from("add_out"),
                name: "sum".to_owned(),
                pin_type: PinType::Float,
                direction: Direction::Output,
            }],
            properties: Vec::new(),
        }
    }

    fn create(
        &self,
        _properties: &HashMap<String, serde_json::Value>,
    ) -> AqueductResult<Box<dyn NodeEvaluator>> {
        Ok(Box::new(AddEvaluator))
    }
}

struct AddEvaluator;

impl NodeEvaluator for AddEvaluator {
    fn evaluate(&mut self, inputs: &[PinValue], _tick: u64) -> AqueductResult<NodeEvalResult> {
        if inputs.len() != 2 {
            return Err(AqueductError::new(
                ErrorKind::Node,
                "TEST_ADD_INPUT_LENGTH",
                "add ノードの入力数が不正です",
            ));
        }

        let left = pin_value_to_f64(&inputs[0])?;
        let right = pin_value_to_f64(&inputs[1])?;

        Ok(NodeEvalResult::Ready(vec![PinValue::Float(left + right)]))
    }

    fn apply_property_patch(
        &mut self,
        _key: &str,
        _value: &serde_json::Value,
    ) -> AqueductResult<()> {
        Ok(())
    }
}

struct RelayFactory {
    type_name: &'static str,
    input_pin: PinId,
    output_pin: PinId,
}

impl RelayFactory {
    fn new(type_name: &'static str, input_pin: PinId, output_pin: PinId) -> Self {
        Self {
            type_name,
            input_pin,
            output_pin,
        }
    }
}

impl NodeFactory for RelayFactory {
    fn node_def(&self) -> NodeDef {
        NodeDef {
            type_name: self.type_name.to_owned(),
            inputs: vec![PinDef {
                id: self.input_pin.clone(),
                name: "in".to_owned(),
                pin_type: PinType::Float,
                direction: Direction::Input,
            }],
            outputs: vec![PinDef {
                id: self.output_pin.clone(),
                name: "out".to_owned(),
                pin_type: PinType::Float,
                direction: Direction::Output,
            }],
            properties: Vec::new(),
        }
    }

    fn create(
        &self,
        _properties: &HashMap<String, serde_json::Value>,
    ) -> AqueductResult<Box<dyn NodeEvaluator>> {
        Ok(Box::new(RelayEvaluator))
    }
}

struct RelayEvaluator;

impl NodeEvaluator for RelayEvaluator {
    fn evaluate(&mut self, inputs: &[PinValue], _tick: u64) -> AqueductResult<NodeEvalResult> {
        let output = inputs.first().cloned().unwrap_or(PinValue::None);
        Ok(NodeEvalResult::Ready(vec![output]))
    }

    fn apply_property_patch(
        &mut self,
        _key: &str,
        _value: &serde_json::Value,
    ) -> AqueductResult<()> {
        Ok(())
    }
}

struct StatefulFactory;

impl NodeFactory for StatefulFactory {
    fn node_def(&self) -> NodeDef {
        NodeDef {
            type_name: "stateful.counter".to_owned(),
            inputs: Vec::new(),
            outputs: vec![PinDef {
                id: PinId::from("state_out"),
                name: "out".to_owned(),
                pin_type: PinType::Float,
                direction: Direction::Output,
            }],
            properties: Vec::new(),
        }
    }

    fn create(
        &self,
        properties: &HashMap<String, serde_json::Value>,
    ) -> AqueductResult<Box<dyn NodeEvaluator>> {
        let value = properties
            .get("value")
            .and_then(serde_json::Value::as_f64)
            .unwrap_or(0.0);

        Ok(Box::new(StatefulEvaluator { value, ticks: 0.0 }))
    }
}

struct StatefulEvaluator {
    value: f64,
    ticks: f64,
}

impl NodeEvaluator for StatefulEvaluator {
    fn evaluate(&mut self, _inputs: &[PinValue], _tick: u64) -> AqueductResult<NodeEvalResult> {
        self.ticks += 1.0;
        Ok(NodeEvalResult::Ready(vec![PinValue::Float(
            self.value + self.ticks,
        )]))
    }

    fn apply_property_patch(&mut self, key: &str, value: &serde_json::Value) -> AqueductResult<()> {
        if key != "value" {
            return Ok(());
        }

        let Some(updated) = value.as_f64() else {
            return Err(AqueductError::new(
                ErrorKind::Node,
                "TEST_STATEFUL_PATCH_INVALID",
                "value は f64 である必要があります",
            ));
        };
        self.value = updated;
        Ok(())
    }
}

struct AsyncEmitterFactory;

impl NodeFactory for AsyncEmitterFactory {
    fn node_def(&self) -> NodeDef {
        NodeDef {
            type_name: "async.emitter".to_owned(),
            inputs: Vec::new(),
            outputs: vec![PinDef {
                id: PinId::from("async_out"),
                name: "out".to_owned(),
                pin_type: PinType::Float,
                direction: Direction::Output,
            }],
            properties: Vec::new(),
        }
    }

    fn create(
        &self,
        _properties: &HashMap<String, serde_json::Value>,
    ) -> AqueductResult<Box<dyn NodeEvaluator>> {
        Ok(Box::new(AsyncEmitterEvaluator { next_job_id: 42 }))
    }
}

struct AsyncEmitterEvaluator {
    next_job_id: u64,
}

impl NodeEvaluator for AsyncEmitterEvaluator {
    fn evaluate(&mut self, _inputs: &[PinValue], _tick: u64) -> AqueductResult<NodeEvalResult> {
        let job_id = self.next_job_id;
        self.next_job_id = self.next_job_id.saturating_add(1);
        Ok(NodeEvalResult::Spawned { job_id })
    }

    fn apply_property_patch(
        &mut self,
        _key: &str,
        _value: &serde_json::Value,
    ) -> AqueductResult<()> {
        Ok(())
    }
}

#[test]
fn topological_sort_works_for_dag() {
    let registry = must_ok(build_test_registry());
    let graph = build_add_graph(1.0, 2.0);

    let order = must_ok(topological_sort(&graph));

    assert_eq!(order.len(), 3);
    let source_a = NodeId::from("source_a");
    let source_b = NodeId::from("source_b");
    let add = NodeId::from("adder");

    let index_a = find_index(&order, &source_a);
    let index_b = find_index(&order, &source_b);
    let index_add = find_index(&order, &add);

    assert!(index_a < index_add);
    assert!(index_b < index_add);

    let graph_compiler = GraphCompiler::new(&registry);
    let compiled_graph = must_ok(graph_compiler.compile(&graph, 0));
    assert_eq!(compiled_graph.eval_order(), order.as_slice());
}

#[test]
fn cycle_detection_returns_true() {
    let mut registry = NodeRegistry::new();
    must_ok(registry.register(Arc::new(RelayFactory::new(
        "relay.a",
        PinId::from("relay_a_in"),
        PinId::from("relay_a_out"),
    ))));
    must_ok(registry.register(Arc::new(RelayFactory::new(
        "relay.b",
        PinId::from("relay_b_in"),
        PinId::from("relay_b_out"),
    ))));

    let node_a = NodeInstance {
        id: NodeId::from("node_a"),
        type_name: "relay.a".to_owned(),
        properties: HashMap::new(),
        position: (0.0, 0.0),
    };
    let node_b = NodeInstance {
        id: NodeId::from("node_b"),
        type_name: "relay.b".to_owned(),
        properties: HashMap::new(),
        position: (100.0, 0.0),
    };

    let mut nodes = HashMap::new();
    let _ = nodes.insert(node_a.id.clone(), node_a);
    let _ = nodes.insert(node_b.id.clone(), node_b);

    let graph = Graph {
        nodes,
        edges: vec![
            Edge {
                id: EdgeId::from("edge_ab"),
                from_node: NodeId::from("node_a"),
                from_pin: PinId::from("relay_a_out"),
                to_node: NodeId::from("node_b"),
                to_pin: PinId::from("relay_b_in"),
            },
            Edge {
                id: EdgeId::from("edge_ba"),
                from_node: NodeId::from("node_b"),
                from_pin: PinId::from("relay_b_out"),
                to_node: NodeId::from("node_a"),
                to_pin: PinId::from("relay_a_in"),
            },
        ],
    };

    let has_cycle = must_ok(detect_cycle(&graph));
    assert!(has_cycle);

    let graph_compiler = GraphCompiler::new(&registry);
    let compile_result = graph_compiler.compile(&graph, 0);
    assert!(compile_result.is_err());
}

#[test]
fn pin_store_value_and_event_handling() {
    let mut store = PinStore::new();
    let value_pin = scoped_pin("node", "value_pin");
    let event_pin = scoped_pin("node", "event_pin");

    store.set_value(value_pin.clone(), PinValue::Int(42));
    let read = store.get_value(&value_pin);
    assert_eq!(read, Some(&PinValue::Int(42)));

    assert!(!store.is_event_fired(&event_pin));
    store.fire_event(event_pin.clone());
    assert!(store.is_event_fired(&event_pin));

    store.clear_events();
    assert!(!store.is_event_fired(&event_pin));
}

#[test]
fn compile_and_tick_integration_for_add_node() {
    let registry = must_ok(build_test_registry());
    let graph = build_add_graph(1.25, 2.75);
    let graph_compiler = GraphCompiler::new(&registry);

    let mut compiled_graph = must_ok(graph_compiler.compile(&graph, 0));
    must_ok(compiled_graph.tick(0));

    let output = compiled_graph.pin_value(&scoped_pin("adder", "add_out"));
    assert_eq!(output, Some(PinValue::Float(4.0)));
}

#[test]
fn hot_patch_replaces_graph_with_arc_swap_on_recompile_path() {
    let registry = must_ok(build_test_registry());
    let graph_compiler = GraphCompiler::new(&registry);
    let initial_graph = build_add_graph(1.0, 2.0);
    let compiled_graph = must_ok(graph_compiler.compile(&initial_graph, 0));
    let live_graph = Arc::new(LiveGraph::new(Arc::new(Mutex::new(compiled_graph))));
    let mut driver = TickDriver::new(Arc::clone(&live_graph));

    let _ = must_ok(driver.run_tick());
    let before = read_live_pin(&live_graph, "adder", "add_out");
    assert_eq!(before, Some(PinValue::Float(3.0)));

    let patcher = GraphPatcher::new(&registry);
    let old_snapshot = live_graph.snapshot();

    let mutations = vec![GraphMutation::AddNode {
        instance: NodeInstance {
            id: NodeId::from("source_c"),
            type_name: "const.a".to_owned(),
            properties: HashMap::from([(String::from("value"), json!(5.0))]),
            position: (300.0, 0.0),
        },
    }];

    let report = must_ok(patcher.patch_live_graph(&live_graph, &mutations));
    assert_eq!(report.graph_rev, 1);

    let new_snapshot = live_graph.snapshot();
    assert!(!Arc::ptr_eq(&old_snapshot, &new_snapshot));
}

#[test]
fn update_property_patch_skips_recompile_and_preserves_evaluator_state() {
    let mut registry = NodeRegistry::new();
    must_ok(registry.register(Arc::new(StatefulFactory)));

    let node = NodeInstance {
        id: NodeId::from("stateful"),
        type_name: "stateful.counter".to_owned(),
        properties: HashMap::from([(String::from("value"), json!(1.0))]),
        position: (0.0, 0.0),
    };

    let mut nodes = HashMap::new();
    let _ = nodes.insert(node.id.clone(), node);
    let graph = Graph {
        nodes,
        edges: Vec::new(),
    };

    let compiler = GraphCompiler::new(&registry);
    let compiled = must_ok(compiler.compile(&graph, 0));
    let live_graph = Arc::new(LiveGraph::new(Arc::new(Mutex::new(compiled))));
    let mut driver = TickDriver::new(Arc::clone(&live_graph));

    let _ = must_ok(driver.run_tick());
    assert_eq!(
        read_live_pin(&live_graph, "stateful", "state_out"),
        Some(PinValue::Float(2.0))
    );

    let patcher = GraphPatcher::new(&registry);
    let old_snapshot = live_graph.snapshot();
    let mutations = vec![GraphMutation::UpdateProperty {
        node_id: NodeId::from("stateful"),
        key: "value".to_owned(),
        value: json!(10.0),
    }];
    let report = must_ok(patcher.patch_live_graph(&live_graph, &mutations));
    assert_eq!(report.graph_rev, 0);

    let new_snapshot = live_graph.snapshot();
    assert!(Arc::ptr_eq(&old_snapshot, &new_snapshot));

    let _ = must_ok(driver.run_tick());
    assert_eq!(
        read_live_pin(&live_graph, "stateful", "state_out"),
        Some(PinValue::Float(12.0))
    );
}

#[test]
fn pin_id_collision_is_avoided_for_same_node_type_instances() {
    let mut registry = NodeRegistry::new();
    must_ok(registry.register(Arc::new(ConstFactory::new(
        "const.shared",
        PinId::from("out"),
    ))));
    must_ok(registry.register(Arc::new(AddFactory)));

    let source_left = NodeInstance {
        id: NodeId::from("source_left"),
        type_name: "const.shared".to_owned(),
        properties: HashMap::from([(String::from("value"), json!(1.5))]),
        position: (0.0, 0.0),
    };
    let source_right = NodeInstance {
        id: NodeId::from("source_right"),
        type_name: "const.shared".to_owned(),
        properties: HashMap::from([(String::from("value"), json!(2.5))]),
        position: (0.0, 100.0),
    };
    let adder = NodeInstance {
        id: NodeId::from("adder"),
        type_name: "math.add".to_owned(),
        properties: HashMap::new(),
        position: (200.0, 50.0),
    };

    let mut nodes = HashMap::new();
    let _ = nodes.insert(source_left.id.clone(), source_left);
    let _ = nodes.insert(source_right.id.clone(), source_right);
    let _ = nodes.insert(adder.id.clone(), adder);

    let graph = Graph {
        nodes,
        edges: vec![
            Edge {
                id: EdgeId::from("edge_left"),
                from_node: NodeId::from("source_left"),
                from_pin: PinId::from("out"),
                to_node: NodeId::from("adder"),
                to_pin: PinId::from("add_in_a"),
            },
            Edge {
                id: EdgeId::from("edge_right"),
                from_node: NodeId::from("source_right"),
                from_pin: PinId::from("out"),
                to_node: NodeId::from("adder"),
                to_pin: PinId::from("add_in_b"),
            },
        ],
    };

    let compiler = GraphCompiler::new(&registry);
    let mut compiled = must_ok(compiler.compile(&graph, 0));
    must_ok(compiled.tick(0));

    assert_eq!(
        compiled.pin_value(&scoped_pin("adder", "add_out")),
        Some(PinValue::Float(4.0))
    );
}

#[test]
fn async_job_complete_then_drain_reflects_to_pin_store() {
    let mut registry = NodeRegistry::new();
    must_ok(registry.register(Arc::new(AsyncEmitterFactory)));

    let worker = NodeInstance {
        id: NodeId::from("worker"),
        type_name: "async.emitter".to_owned(),
        properties: HashMap::new(),
        position: (0.0, 0.0),
    };

    let mut nodes = HashMap::new();
    let _ = nodes.insert(worker.id.clone(), worker);
    let graph = Graph {
        nodes,
        edges: Vec::new(),
    };

    let compiler = GraphCompiler::new(&registry);
    let mut compiled = must_ok(compiler.compile(&graph, 0));

    must_ok(compiled.tick(0));
    assert_eq!(compiled.pending_job_count(), 1);

    let accepted = compiled.complete_job(AsyncJobResult {
        job_id: 42,
        node_id: NodeId::from("worker"),
        tick_spawned: 0,
        outputs: vec![PinValue::Float(9.0)],
    });
    assert!(accepted);

    must_ok(compiled.tick(1));
    assert_eq!(
        compiled.pin_value(&scoped_pin("worker", "async_out")),
        Some(PinValue::Float(9.0))
    );
}

#[test]
fn deterministic_output_for_same_input_sequence() {
    let registry = must_ok(build_test_registry());
    let graph = build_add_graph(10.0, 5.0);

    let first = must_ok(run_outputs(&registry, &graph, 6));
    let second = must_ok(run_outputs(&registry, &graph, 6));

    assert_eq!(first, second);
}

#[test]
fn apply_mutations_updates_graph() {
    let mut graph = build_add_graph(1.0, 2.0);

    let mutation = GraphMutation::UpdateProperty {
        node_id: NodeId::from("source_a"),
        key: "value".to_owned(),
        value: json!(9.0),
    };

    let changed = must_ok(apply_mutations(&mut graph, &[mutation]));
    assert!(changed.contains(&NodeId::from("source_a")));

    let Some(node) = graph.nodes.get(&NodeId::from("source_a")) else {
        panic!("source_a が見つかりません");
    };

    assert_eq!(node.properties.get("value"), Some(&json!(9.0)));
}

fn build_test_registry() -> AqueductResult<NodeRegistry> {
    let mut registry = NodeRegistry::new();
    registry.register(Arc::new(ConstFactory::new(
        "const.a",
        PinId::from("src_a_out"),
    )))?;
    registry.register(Arc::new(ConstFactory::new(
        "const.b",
        PinId::from("src_b_out"),
    )))?;
    registry.register(Arc::new(AddFactory))?;
    Ok(registry)
}

fn build_add_graph(value_a: f64, value_b: f64) -> Graph {
    let source_a = NodeInstance {
        id: NodeId::from("source_a"),
        type_name: "const.a".to_owned(),
        properties: HashMap::from([(String::from("value"), json!(value_a))]),
        position: (0.0, 0.0),
    };
    let source_b = NodeInstance {
        id: NodeId::from("source_b"),
        type_name: "const.b".to_owned(),
        properties: HashMap::from([(String::from("value"), json!(value_b))]),
        position: (0.0, 100.0),
    };
    let adder = NodeInstance {
        id: NodeId::from("adder"),
        type_name: "math.add".to_owned(),
        properties: HashMap::new(),
        position: (200.0, 50.0),
    };

    let mut nodes = HashMap::new();
    let _ = nodes.insert(source_a.id.clone(), source_a);
    let _ = nodes.insert(source_b.id.clone(), source_b);
    let _ = nodes.insert(adder.id.clone(), adder);

    Graph {
        nodes,
        edges: vec![
            Edge {
                id: EdgeId::from("edge_a"),
                from_node: NodeId::from("source_a"),
                from_pin: PinId::from("src_a_out"),
                to_node: NodeId::from("adder"),
                to_pin: PinId::from("add_in_a"),
            },
            Edge {
                id: EdgeId::from("edge_b"),
                from_node: NodeId::from("source_b"),
                from_pin: PinId::from("src_b_out"),
                to_node: NodeId::from("adder"),
                to_pin: PinId::from("add_in_b"),
            },
        ],
    }
}

fn run_outputs(
    registry: &NodeRegistry,
    graph: &Graph,
    ticks: usize,
) -> AqueductResult<Vec<PinValue>> {
    let graph_compiler = GraphCompiler::new(registry);
    let mut compiled_graph = graph_compiler.compile(graph, 0)?;

    let mut outputs = Vec::with_capacity(ticks);
    for tick in 0..ticks {
        let tick = u64::try_from(tick).map_err(|_error| {
            AqueductError::new(
                ErrorKind::Runtime,
                "TEST_TICK_CONVERSION_FAILED",
                "tick 値を u64 へ変換できませんでした",
            )
        })?;
        compiled_graph.tick(tick)?;
        let value = compiled_graph
            .pin_value(&scoped_pin("adder", "add_out"))
            .unwrap_or(PinValue::None);
        outputs.push(value);
    }

    Ok(outputs)
}

fn read_live_pin(live_graph: &LiveGraph, node_id: &str, pin_id: &str) -> Option<PinValue> {
    must_ok(live_graph.with_graph(|graph| Ok(graph.pin_value(&scoped_pin(node_id, pin_id)))))
}

fn scoped_pin(node_id: &str, pin_id: &str) -> ScopedPinId {
    ScopedPinId::new(NodeId::from(node_id), PinId::from(pin_id))
}

fn find_index(order: &[NodeId], id: &NodeId) -> usize {
    for (index, current) in order.iter().enumerate() {
        if current == id {
            return index;
        }
    }

    panic!("ノード {id} が評価順に存在しません");
}

fn pin_value_to_f64(value: &PinValue) -> AqueductResult<f64> {
    if let PinValue::Float(number) = value {
        return Ok(*number);
    }

    Err(AqueductError::new(
        ErrorKind::Node,
        "TEST_ADD_INPUT_TYPE",
        "add ノード入力は Float である必要があります",
    ))
}

fn must_ok<T>(result: AqueductResult<T>) -> T {
    match result {
        Ok(value) => value,
        Err(error) => panic!("unexpected error: {error}"),
    }
}
