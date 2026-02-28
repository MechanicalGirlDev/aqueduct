use std::collections::HashMap;
use std::sync::Arc;

use aqueduct_core::{
    AqueductError, AqueductResult, ErrorKind, NodeEvalResult, NodeEvaluator, NodeFactory,
};
use aqueduct_protocol::{NodeDef, PinType, PinValue, Property};

use crate::{
    any_input, boxed, input_pin, output_pin, pin_value_to_text, string_input,
    usize_to_i64_saturating, StatelessFactory,
};

pub(crate) fn factories() -> Vec<Arc<dyn NodeFactory>> {
    vec![
        boxed(StatelessFactory::new(
            node_def(
                "string.concat",
                vec![
                    input_pin("a", PinType::String),
                    input_pin("b", PinType::String),
                ],
                vec![output_pin("out", PinType::String)],
            ),
            |inputs, _tick| {
                let left = string_input(inputs, 0);
                let right = string_input(inputs, 1);
                vec![PinValue::String(left + &right)]
            },
        )),
        boxed(StatelessFactory::new(
            node_def(
                "string.length",
                vec![input_pin("s", PinType::String)],
                vec![output_pin("out", PinType::Int)],
            ),
            |inputs, _tick| {
                let text = string_input(inputs, 0);
                vec![PinValue::Int(usize_to_i64_saturating(text.chars().count()))]
            },
        )),
        boxed(StatelessFactory::new(
            node_def(
                "string.uppercase",
                vec![input_pin("s", PinType::String)],
                vec![output_pin("out", PinType::String)],
            ),
            |inputs, _tick| {
                let text = string_input(inputs, 0);
                vec![PinValue::String(text.to_uppercase())]
            },
        )),
        boxed(StatelessFactory::new(
            node_def(
                "string.lowercase",
                vec![input_pin("s", PinType::String)],
                vec![output_pin("out", PinType::String)],
            ),
            |inputs, _tick| {
                let text = string_input(inputs, 0);
                vec![PinValue::String(text.to_lowercase())]
            },
        )),
        boxed(StatelessFactory::new(
            node_def(
                "string.contains",
                vec![
                    input_pin("haystack", PinType::String),
                    input_pin("needle", PinType::String),
                ],
                vec![output_pin("out", PinType::Bool)],
            ),
            |inputs, _tick| {
                let haystack = string_input(inputs, 0);
                let needle = string_input(inputs, 1);
                vec![PinValue::Bool(haystack.contains(&needle))]
            },
        )),
        boxed(StatelessFactory::new(
            node_def(
                "string.replace",
                vec![
                    input_pin("s", PinType::String),
                    input_pin("from", PinType::String),
                    input_pin("to", PinType::String),
                ],
                vec![output_pin("out", PinType::String)],
            ),
            |inputs, _tick| {
                let source = string_input(inputs, 0);
                let from = string_input(inputs, 1);
                let to = string_input(inputs, 2);
                vec![PinValue::String(source.replace(&from, &to))]
            },
        )),
        boxed(StatelessFactory::new(
            node_def(
                "string.trim",
                vec![input_pin("s", PinType::String)],
                vec![output_pin("out", PinType::String)],
            ),
            |inputs, _tick| {
                let source = string_input(inputs, 0);
                vec![PinValue::String(source.trim().to_owned())]
            },
        )),
        boxed(StringFormatFactory),
    ]
}

struct StringFormatFactory;

impl NodeFactory for StringFormatFactory {
    fn node_def(&self) -> NodeDef {
        NodeDef {
            type_name: String::from("string.format"),
            inputs: vec![
                input_pin("template", PinType::String),
                input_pin("arg", PinType::Any),
            ],
            outputs: vec![output_pin("out", PinType::String)],
            properties: vec![Property {
                key: String::from("template"),
                name: String::from("template"),
                description: Some(String::from("未接続時に使うテンプレート文字列")),
                default_value: serde_json::Value::String(String::new()),
            }],
        }
    }

    fn create(
        &self,
        properties: &HashMap<String, serde_json::Value>,
    ) -> AqueductResult<Box<dyn NodeEvaluator>> {
        let template = properties
            .get("template")
            .map(parse_template)
            .transpose()?
            .unwrap_or_default();
        Ok(Box::new(StringFormatEvaluator { template }))
    }
}

struct StringFormatEvaluator {
    template: String,
}

impl NodeEvaluator for StringFormatEvaluator {
    fn evaluate(&mut self, inputs: &[PinValue], _tick: u64) -> AqueductResult<NodeEvalResult> {
        let template_input = string_input(inputs, 0);
        let template = if template_input.is_empty() {
            self.template.as_str()
        } else {
            template_input.as_str()
        };
        let arg = any_input(inputs, 1);
        let arg_text = pin_value_to_text(&arg);

        Ok(NodeEvalResult::Ready(vec![PinValue::String(
            template.replace("{}", &arg_text),
        )]))
    }

    fn apply_property_patch(&mut self, key: &str, value: &serde_json::Value) -> AqueductResult<()> {
        if key != "template" {
            return Ok(());
        }

        self.template = parse_template(value)?;
        Ok(())
    }
}

fn parse_template(value: &serde_json::Value) -> AqueductResult<String> {
    if let Some(template) = value.as_str() {
        return Ok(template.to_owned());
    }

    Err(AqueductError::new(
        ErrorKind::Node,
        "STRING_FORMAT_INVALID_TEMPLATE",
        "template は文字列である必要があります",
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
