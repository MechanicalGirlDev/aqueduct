use std::sync::Arc;

use aqueduct_core::NodeFactory;
use aqueduct_protocol::{NodeDef, PinType, PinValue};

use crate::{
    any_input, bool_input, boxed, float_input, input_pin, int_input, output_pin, string_input,
    StatelessFactory,
};

pub(crate) fn factories() -> Vec<Arc<dyn NodeFactory>> {
    vec![
        boxed(StatelessFactory::new(
            node_def(
                "convert.float_to_int",
                vec![input_pin("a", PinType::Float)],
                vec![output_pin("out", PinType::Int)],
            ),
            |inputs, _tick| {
                let value = float_input(inputs, 0);
                vec![PinValue::Int(float_to_i64(value))]
            },
        )),
        boxed(StatelessFactory::new(
            node_def(
                "convert.int_to_float",
                vec![input_pin("a", PinType::Int)],
                vec![output_pin("out", PinType::Float)],
            ),
            |inputs, _tick| {
                let value = int_input(inputs, 0);
                vec![PinValue::Float(int_to_f64(value))]
            },
        )),
        boxed(StatelessFactory::new(
            node_def(
                "convert.to_string",
                vec![input_pin("a", PinType::Any)],
                vec![output_pin("out", PinType::String)],
            ),
            |inputs, _tick| {
                let value = any_input(inputs, 0);
                vec![PinValue::String(format!("{value:?}"))]
            },
        )),
        boxed(StatelessFactory::new(
            node_def(
                "convert.parse_float",
                vec![input_pin("s", PinType::String)],
                vec![output_pin("out", PinType::Float)],
            ),
            |inputs, _tick| {
                let text = string_input(inputs, 0);
                let parsed = text.parse::<f64>().unwrap_or(0.0);
                vec![PinValue::Float(parsed)]
            },
        )),
        boxed(StatelessFactory::new(
            node_def(
                "convert.parse_int",
                vec![input_pin("s", PinType::String)],
                vec![output_pin("out", PinType::Int)],
            ),
            |inputs, _tick| {
                let text = string_input(inputs, 0);
                let parsed = text.parse::<i64>().unwrap_or(0);
                vec![PinValue::Int(parsed)]
            },
        )),
        boxed(StatelessFactory::new(
            node_def(
                "convert.bool_to_int",
                vec![input_pin("a", PinType::Bool)],
                vec![output_pin("out", PinType::Int)],
            ),
            |inputs, _tick| {
                let value = bool_input(inputs, 0);
                vec![PinValue::Int(i64::from(value))]
            },
        )),
    ]
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap
)]
fn float_to_i64(value: f64) -> i64 {
    value as i64
}

#[allow(clippy::cast_precision_loss)]
fn int_to_f64(value: i64) -> f64 {
    value as f64
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
