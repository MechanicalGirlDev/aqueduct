#![forbid(unsafe_code)]
#![deny(clippy::all, clippy::pedantic)]

//! Aqueduct コアフレームワーク。

/// グラフコンパイル。
pub mod compiler;
/// エラー型。
pub mod error;
/// グラフ検証とアルゴリズム。
pub mod graph;
/// ノード抽象。
pub mod node;
/// パッチ適用。
pub mod patch;
/// ピンストア。
pub mod pin_store;
/// ノードレジストリ。
pub mod registry;
/// ランタイム。
pub mod runtime;

pub use aqueduct_protocol as protocol;

pub use compiler::{AsyncJobResult, CompiledGraph, CompiledNode, GraphCompiler};
pub use error::{AqueductError, AqueductResult, ErrorKind};
pub use graph::{detect_cycle, downstream_nodes, topological_sort, validate_graph};
pub use node::{NodeEvalResult, NodeEvaluator, NodeFactory};
pub use patch::{apply_mutations, GraphPatcher, PatchReport};
pub use pin_store::{PinStore, ScopedPinId};
pub use registry::NodeRegistry;
pub use runtime::{LiveGraph, TickDriver};
