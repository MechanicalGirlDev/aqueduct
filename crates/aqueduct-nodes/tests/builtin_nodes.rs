use std::collections::HashMap;

use aqueduct_core::{CompiledGraph, GraphCompiler, NodeRegistry, ScopedPinId};
use aqueduct_nodes::register_all;
use aqueduct_protocol::{Edge, EdgeId, Graph, NodeId, NodeInstance, PinId, PinValue};

#[test]
fn math_add_works() {
    let registry = build_registry();
    let graph = graph(vec![node("add", "math.add")], Vec::new());
    let mut compiled = compile(&registry, &graph);

    compiled.set_pin_value(scoped_pin("add", "a"), PinValue::Float(1.5));
    compiled.set_pin_value(scoped_pin("add", "b"), PinValue::Float(2.25));

    compiled.tick(0).expect("tick failed");

    assert_eq!(
        compiled.pin_value(&scoped_pin("add", "out")),
        Some(PinValue::Float(3.75))
    );
}

#[test]
fn string_concat_and_length_work() {
    let registry = build_registry();
    let graph = graph(
        vec![
            node("concat", "string.concat"),
            node("length", "string.length"),
        ],
        vec![edge("e1", "concat", "out", "length", "s")],
    );
    let mut compiled = compile(&registry, &graph);

    compiled.set_pin_value(
        scoped_pin("concat", "a"),
        PinValue::String(String::from("Hello")),
    );
    compiled.set_pin_value(
        scoped_pin("concat", "b"),
        PinValue::String(String::from("世界")),
    );

    compiled.tick(0).expect("tick failed");

    assert_eq!(
        compiled.pin_value(&scoped_pin("concat", "out")),
        Some(PinValue::String(String::from("Hello世界")))
    );
    assert_eq!(
        compiled.pin_value(&scoped_pin("length", "out")),
        Some(PinValue::Int(7))
    );
}

#[test]
fn logic_and_and_not_work() {
    let registry = build_registry();
    let graph = graph(
        vec![node("and", "logic.and"), node("not", "logic.not")],
        vec![edge("e1", "and", "out", "not", "a")],
    );
    let mut compiled = compile(&registry, &graph);

    compiled.set_pin_value(scoped_pin("and", "a"), PinValue::Bool(true));
    compiled.set_pin_value(scoped_pin("and", "b"), PinValue::Bool(false));

    compiled.tick(0).expect("tick failed");

    assert_eq!(
        compiled.pin_value(&scoped_pin("and", "out")),
        Some(PinValue::Bool(false))
    );
    assert_eq!(
        compiled.pin_value(&scoped_pin("not", "out")),
        Some(PinValue::Bool(true))
    );
}

#[test]
fn convert_float_to_int_and_back_work() {
    let registry = build_registry();
    let graph = graph(
        vec![
            node("float_to_int", "convert.float_to_int"),
            node("int_to_float", "convert.int_to_float"),
        ],
        vec![edge("e1", "float_to_int", "out", "int_to_float", "a")],
    );
    let mut compiled = compile(&registry, &graph);

    compiled.set_pin_value(scoped_pin("float_to_int", "a"), PinValue::Float(42.9));
    compiled.tick(0).expect("tick failed");

    assert_eq!(
        compiled.pin_value(&scoped_pin("float_to_int", "out")),
        Some(PinValue::Int(42))
    );
    assert_eq!(
        compiled.pin_value(&scoped_pin("int_to_float", "out")),
        Some(PinValue::Float(42.0))
    );
}

#[test]
fn time_tick_outputs_current_tick() {
    let registry = build_registry();
    let graph = graph(vec![node("clock", "time.tick")], Vec::new());
    let mut compiled = compile(&registry, &graph);

    compiled.tick(0).expect("tick failed");
    assert_eq!(
        compiled.pin_value(&scoped_pin("clock", "tick")),
        Some(PinValue::Int(0))
    );

    compiled.tick(1).expect("tick failed");
    assert_eq!(
        compiled.pin_value(&scoped_pin("clock", "tick")),
        Some(PinValue::Int(1))
    );
}

#[test]
fn register_all_registers_all_builtin_nodes() {
    let mut registry = NodeRegistry::new();
    register_all(&mut registry).expect("register_all failed");

    let expected_types = [
        "math.add",
        "math.subtract",
        "math.multiply",
        "math.divide",
        "math.negate",
        "math.abs",
        "math.min",
        "math.max",
        "math.clamp",
        "math.sin",
        "math.cos",
        "math.modulo",
        "string.concat",
        "string.length",
        "string.uppercase",
        "string.lowercase",
        "string.contains",
        "string.replace",
        "string.trim",
        "string.format",
        "logic.and",
        "logic.or",
        "logic.not",
        "logic.equals",
        "logic.greater_than",
        "logic.less_than",
        "logic.select",
        "time.tick",
        "time.elapsed",
        "convert.float_to_int",
        "convert.int_to_float",
        "convert.to_string",
        "convert.parse_float",
        "convert.parse_int",
        "convert.bool_to_int",
    ];

    assert_eq!(registry.list_node_defs().len(), expected_types.len());

    for type_name in expected_types {
        assert!(
            registry.contains(type_name),
            "missing node type: {type_name}"
        );
    }
}

fn build_registry() -> NodeRegistry {
    let mut registry = NodeRegistry::new();
    register_all(&mut registry).expect("register_all failed");
    registry
}

fn compile(registry: &NodeRegistry, graph: &Graph) -> CompiledGraph {
    GraphCompiler::new(registry)
        .compile(graph, 0)
        .expect("compile failed")
}

fn node(id: &str, type_name: &str) -> NodeInstance {
    NodeInstance {
        id: NodeId::from(id),
        type_name: type_name.to_owned(),
        properties: HashMap::new(),
        position: (0.0, 0.0),
    }
}

fn edge(id: &str, from_node: &str, from_pin: &str, to_node: &str, to_pin: &str) -> Edge {
    Edge {
        id: EdgeId::from(id),
        from_node: NodeId::from(from_node),
        from_pin: PinId::from(from_pin),
        to_node: NodeId::from(to_node),
        to_pin: PinId::from(to_pin),
    }
}

fn graph(nodes: Vec<NodeInstance>, edges: Vec<Edge>) -> Graph {
    let nodes = nodes
        .into_iter()
        .map(|instance| (instance.id.clone(), instance))
        .collect();

    Graph { nodes, edges }
}

fn scoped_pin(node_id: &str, pin_id: &str) -> ScopedPinId {
    ScopedPinId::new(NodeId::from(node_id), PinId::from(pin_id))
}
