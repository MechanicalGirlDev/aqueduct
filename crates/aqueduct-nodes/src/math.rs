use std::sync::Arc;

use aqueduct_core::NodeFactory;
use aqueduct_protocol::{NodeDef, PinType, PinValue};

use crate::{boxed, float_input, input_pin, output_pin, StatelessFactory};

pub(crate) fn factories() -> Vec<Arc<dyn NodeFactory>> {
    vec![
        binary_float_factory("math.add", |a, b| a + b),
        binary_float_factory("math.subtract", |a, b| a - b),
        binary_float_factory("math.multiply", |a, b| a * b),
        binary_float_factory("math.divide", |a, b| if b == 0.0 { 0.0 } else { a / b }),
        unary_float_factory("math.negate", |a| -a),
        unary_float_factory("math.abs", f64::abs),
        binary_float_factory("math.min", f64::min),
        binary_float_factory("math.max", f64::max),
        boxed(StatelessFactory::new(
            node_def(
                "math.clamp",
                vec![
                    input_pin("value", PinType::Float),
                    input_pin("min", PinType::Float),
                    input_pin("max", PinType::Float),
                ],
                vec![output_pin("out", PinType::Float)],
            ),
            eval_clamp,
        )),
        unary_float_factory("math.sin", f64::sin),
        unary_float_factory("math.cos", f64::cos),
        binary_float_factory("math.modulo", |a, b| if b == 0.0 { 0.0 } else { a % b }),
    ]
}

fn unary_float_factory(type_name: &str, operation: fn(f64) -> f64) -> Arc<dyn NodeFactory> {
    boxed(StatelessFactory::new(
        node_def(
            type_name,
            vec![input_pin("a", PinType::Float)],
            vec![output_pin("out", PinType::Float)],
        ),
        move |inputs, _tick| {
            let value = float_input(inputs, 0);
            vec![PinValue::Float(operation(value))]
        },
    ))
}

fn binary_float_factory(type_name: &str, operation: fn(f64, f64) -> f64) -> Arc<dyn NodeFactory> {
    boxed(StatelessFactory::new(
        node_def(
            type_name,
            vec![
                input_pin("a", PinType::Float),
                input_pin("b", PinType::Float),
            ],
            vec![output_pin("out", PinType::Float)],
        ),
        move |inputs, _tick| {
            let left = float_input(inputs, 0);
            let right = float_input(inputs, 1);
            vec![PinValue::Float(operation(left, right))]
        },
    ))
}

fn eval_clamp(inputs: &[PinValue], _tick: u64) -> Vec<PinValue> {
    let value = float_input(inputs, 0);
    let min_value = float_input(inputs, 1);
    let max_value = float_input(inputs, 2);

    let (lower, upper) = if min_value <= max_value {
        (min_value, max_value)
    } else {
        (max_value, min_value)
    };

    vec![PinValue::Float(value.clamp(lower, upper))]
}

fn node_def(
    type_name: &str,
    inputs: Vec<aqueduct_protocol::PinDef>,
    outputs: Vec<aqueduct_protocol::PinDef>,
) -> NodeDef {
    NodeDef {
        type_name: type_name.to_owned(),
        inputs,
        outputs,
        properties: Vec::new(),
    }
}
