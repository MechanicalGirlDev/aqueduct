#![forbid(unsafe_code)]
#![deny(clippy::all, clippy::pedantic)]

use std::collections::HashMap;
use std::sync::Arc;

use aqueduct_core::{AqueductResult, NodeEvalResult, NodeEvaluator, NodeFactory, NodeRegistry};
use aqueduct_protocol::{Direction, NodeDef, PinDef, PinId, PinType, PinValue};

pub mod convert;
pub mod logic;
pub mod math;
pub mod string;
pub mod time;

type StatelessEvalFn = Arc<dyn Fn(&[PinValue], u64) -> Vec<PinValue> + Send + Sync>;

#[derive(Clone)]
pub(crate) struct StatelessFactory {
    node_def: NodeDef,
    eval_fn: StatelessEvalFn,
}

impl StatelessFactory {
    #[must_use]
    pub(crate) fn new(
        node_def: NodeDef,
        eval_fn: impl Fn(&[PinValue], u64) -> Vec<PinValue> + Send + Sync + 'static,
    ) -> Self {
        Self {
            node_def,
            eval_fn: Arc::new(eval_fn),
        }
    }
}

impl NodeFactory for StatelessFactory {
    fn node_def(&self) -> NodeDef {
        self.node_def.clone()
    }

    fn create(
        &self,
        _properties: &HashMap<String, serde_json::Value>,
    ) -> AqueductResult<Box<dyn NodeEvaluator>> {
        Ok(Box::new(StatelessEvaluator {
            eval_fn: Arc::clone(&self.eval_fn),
        }))
    }
}

pub(crate) struct StatelessEvaluator {
    eval_fn: StatelessEvalFn,
}

impl NodeEvaluator for StatelessEvaluator {
    fn evaluate(&mut self, inputs: &[PinValue], tick: u64) -> AqueductResult<NodeEvalResult> {
        Ok(NodeEvalResult::Ready((self.eval_fn)(inputs, tick)))
    }

    fn apply_property_patch(
        &mut self,
        _key: &str,
        _value: &serde_json::Value,
    ) -> AqueductResult<()> {
        Ok(())
    }
}

#[must_use]
pub(crate) fn input_pin(id: &str, pin_type: PinType) -> PinDef {
    PinDef {
        id: PinId::from(id),
        name: id.to_owned(),
        pin_type,
        direction: Direction::Input,
    }
}

#[must_use]
pub(crate) fn output_pin(id: &str, pin_type: PinType) -> PinDef {
    PinDef {
        id: PinId::from(id),
        name: id.to_owned(),
        pin_type,
        direction: Direction::Output,
    }
}

#[must_use]
pub(crate) fn float_input(inputs: &[PinValue], index: usize) -> f64 {
    match inputs.get(index) {
        Some(PinValue::Float(value)) => *value,
        _ => 0.0,
    }
}

#[must_use]
pub(crate) fn int_input(inputs: &[PinValue], index: usize) -> i64 {
    match inputs.get(index) {
        Some(PinValue::Int(value)) => *value,
        _ => 0,
    }
}

#[must_use]
pub(crate) fn bool_input(inputs: &[PinValue], index: usize) -> bool {
    match inputs.get(index) {
        Some(PinValue::Bool(value)) => *value,
        _ => false,
    }
}

#[must_use]
pub(crate) fn string_input(inputs: &[PinValue], index: usize) -> String {
    match inputs.get(index) {
        Some(PinValue::String(value)) => value.clone(),
        _ => String::new(),
    }
}

#[must_use]
pub(crate) fn any_input(inputs: &[PinValue], index: usize) -> PinValue {
    inputs.get(index).cloned().unwrap_or(PinValue::None)
}

#[must_use]
pub(crate) fn pin_value_to_text(value: &PinValue) -> String {
    match value {
        PinValue::Float(number) => number.to_string(),
        PinValue::Int(number) => number.to_string(),
        PinValue::Bool(flag) => flag.to_string(),
        PinValue::String(text) => text.clone(),
        PinValue::Json(json) => json.to_string(),
        PinValue::Event => String::from("event"),
        PinValue::None => String::new(),
    }
}

#[must_use]
pub(crate) fn usize_to_i64_saturating(value: usize) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

#[must_use]
pub(crate) fn u64_to_i64_saturating(value: u64) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

/// Register all built-in node factories.
///
/// # Errors
/// Returns an error when registration fails (for example, duplicate type names).
pub fn register_all(registry: &mut NodeRegistry) -> AqueductResult<()> {
    for factory in math::factories() {
        registry.register(factory)?;
    }
    for factory in string::factories() {
        registry.register(factory)?;
    }
    for factory in logic::factories() {
        registry.register(factory)?;
    }
    for factory in time::factories() {
        registry.register(factory)?;
    }
    for factory in convert::factories() {
        registry.register(factory)?;
    }

    Ok(())
}

pub(crate) fn boxed(factory: impl NodeFactory + 'static) -> Arc<dyn NodeFactory> {
    Arc::new(factory)
}
