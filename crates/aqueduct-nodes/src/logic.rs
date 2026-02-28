use std::sync::Arc;

use aqueduct_core::NodeFactory;
use aqueduct_protocol::{NodeDef, PinType, PinValue};

use crate::{any_input, bool_input, boxed, float_input, input_pin, output_pin, StatelessFactory};

pub(crate) fn factories() -> Vec<Arc<dyn NodeFactory>> {
    vec![
        binary_bool_factory("logic.and", |a, b| a && b),
        binary_bool_factory("logic.or", |a, b| a || b),
        unary_bool_factory("logic.not", |a| !a),
        boxed(StatelessFactory::new(
            node_def(
                "logic.equals",
                vec![input_pin("a", PinType::Any), input_pin("b", PinType::Any)],
                vec![output_pin("out", PinType::Bool)],
            ),
            |inputs, _tick| {
                let left = any_input(inputs, 0);
                let right = any_input(inputs, 1);
                vec![PinValue::Bool(left == right)]
            },
        )),
        boxed(StatelessFactory::new(
            node_def(
                "logic.greater_than",
                vec![
                    input_pin("a", PinType::Float),
                    input_pin("b", PinType::Float),
                ],
                vec![output_pin("out", PinType::Bool)],
            ),
            |inputs, _tick| {
                let left = float_input(inputs, 0);
                let right = float_input(inputs, 1);
                vec![PinValue::Bool(left > right)]
            },
        )),
        boxed(StatelessFactory::new(
            node_def(
                "logic.less_than",
                vec![
                    input_pin("a", PinType::Float),
                    input_pin("b", PinType::Float),
                ],
                vec![output_pin("out", PinType::Bool)],
            ),
            |inputs, _tick| {
                let left = float_input(inputs, 0);
                let right = float_input(inputs, 1);
                vec![PinValue::Bool(left < right)]
            },
        )),
        boxed(StatelessFactory::new(
            node_def(
                "logic.select",
                vec![
                    input_pin("condition", PinType::Bool),
                    input_pin("if_true", PinType::Any),
                    input_pin("if_false", PinType::Any),
                ],
                vec![output_pin("out", PinType::Any)],
            ),
            |inputs, _tick| {
                let condition = bool_input(inputs, 0);
                if condition {
                    vec![any_input(inputs, 1)]
                } else {
                    vec![any_input(inputs, 2)]
                }
            },
        )),
    ]
}

fn unary_bool_factory(type_name: &str, operation: fn(bool) -> bool) -> Arc<dyn NodeFactory> {
    boxed(StatelessFactory::new(
        node_def(
            type_name,
            vec![input_pin("a", PinType::Bool)],
            vec![output_pin("out", PinType::Bool)],
        ),
        move |inputs, _tick| {
            let value = bool_input(inputs, 0);
            vec![PinValue::Bool(operation(value))]
        },
    ))
}

fn binary_bool_factory(type_name: &str, operation: fn(bool, bool) -> bool) -> Arc<dyn NodeFactory> {
    boxed(StatelessFactory::new(
        node_def(
            type_name,
            vec![input_pin("a", PinType::Bool), input_pin("b", PinType::Bool)],
            vec![output_pin("out", PinType::Bool)],
        ),
        move |inputs, _tick| {
            let left = bool_input(inputs, 0);
            let right = bool_input(inputs, 1);
            vec![PinValue::Bool(operation(left, right))]
        },
    ))
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
