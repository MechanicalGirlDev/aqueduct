use std::collections::HashMap;
use std::sync::Arc;

use aqueduct_core::{
    AqueductError, AqueductResult, ErrorKind, NodeEvalResult, NodeEvaluator, NodeFactory,
};
use aqueduct_protocol::{NodeDef, PinType, PinValue, Property};

use crate::{boxed, output_pin, u64_to_i64_saturating, StatelessFactory};

pub(crate) fn factories() -> Vec<Arc<dyn NodeFactory>> {
    vec![
        boxed(StatelessFactory::new(
            NodeDef {
                type_name: String::from("time.tick"),
                inputs: Vec::new(),
                outputs: vec![output_pin("tick", PinType::Int)],
                properties: Vec::new(),
            },
            |_inputs, tick| vec![PinValue::Int(u64_to_i64_saturating(tick))],
        )),
        boxed(TimeElapsedFactory),
    ]
}

struct TimeElapsedFactory;

impl NodeFactory for TimeElapsedFactory {
    fn node_def(&self) -> NodeDef {
        NodeDef {
            type_name: String::from("time.elapsed"),
            inputs: Vec::new(),
            outputs: vec![output_pin("elapsed", PinType::Int)],
            properties: vec![Property {
                key: String::from("start_tick"),
                name: String::from("start_tick"),
                description: Some(String::from("elapsed の基準 tick")),
                default_value: serde_json::Value::from(0_u64),
            }],
        }
    }

    fn create(
        &self,
        properties: &HashMap<String, serde_json::Value>,
    ) -> AqueductResult<Box<dyn NodeEvaluator>> {
        let start_tick = properties
            .get("start_tick")
            .map(parse_u64)
            .transpose()?
            .unwrap_or(0);

        Ok(Box::new(TimeElapsedEvaluator { start_tick }))
    }
}

struct TimeElapsedEvaluator {
    start_tick: u64,
}

impl NodeEvaluator for TimeElapsedEvaluator {
    fn evaluate(&mut self, _inputs: &[PinValue], tick: u64) -> AqueductResult<NodeEvalResult> {
        let elapsed = tick.saturating_sub(self.start_tick);
        Ok(NodeEvalResult::Ready(vec![PinValue::Int(
            u64_to_i64_saturating(elapsed),
        )]))
    }

    fn apply_property_patch(&mut self, key: &str, value: &serde_json::Value) -> AqueductResult<()> {
        if key != "start_tick" {
            return Ok(());
        }

        self.start_tick = parse_u64(value)?;
        Ok(())
    }
}

fn parse_u64(value: &serde_json::Value) -> AqueductResult<u64> {
    if let Some(number) = value.as_u64() {
        return Ok(number);
    }

    if let Some(number) = value.as_i64() {
        if let Ok(converted) = u64::try_from(number) {
            return Ok(converted);
        }
    }

    Err(AqueductError::new(
        ErrorKind::Node,
        "TIME_ELAPSED_INVALID_START_TICK",
        "start_tick は 0 以上の整数である必要があります",
    ))
}
