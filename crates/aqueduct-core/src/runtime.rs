//! ランタイム実行モデル。

use std::sync::{Arc, Mutex};

use arc_swap::ArcSwap;

use crate::compiler::CompiledGraph;
use crate::error::{AqueductError, AqueductResult, ErrorKind};

/// 実行中グラフのスナップショット管理。
pub struct LiveGraph {
    /// 現在のコンパイル済みグラフ。
    pub current: ArcSwap<Mutex<CompiledGraph>>,
}

impl LiveGraph {
    /// 初期グラフで作成する。
    #[must_use]
    pub fn new(initial: Arc<Mutex<CompiledGraph>>) -> Self {
        Self {
            current: ArcSwap::from(initial),
        }
    }

    /// 現在グラフを取得する。
    #[must_use]
    pub fn snapshot(&self) -> Arc<Mutex<CompiledGraph>> {
        self.current.load_full()
    }

    /// 新しいグラフへアトミックに差し替える。
    pub fn replace(&self, next: Arc<Mutex<CompiledGraph>>) {
        self.current.store(next);
    }

    /// 現在グラフへ読み取りアクセスする。
    ///
    /// # Errors
    /// ロック破損またはクロージャ実行時エラーを返します。
    pub fn with_graph<R>(
        &self,
        reader: impl FnOnce(&CompiledGraph) -> AqueductResult<R>,
    ) -> AqueductResult<R> {
        let snapshot = self.snapshot();
        let guard = snapshot.lock().map_err(|_error| {
            AqueductError::new(
                ErrorKind::Runtime,
                "LIVE_GRAPH_LOCK_POISONED",
                "LiveGraph ロックが壊れています",
            )
        })?;
        reader(&guard)
    }

    /// 現在グラフへ書き込みアクセスする。
    ///
    /// # Errors
    /// ロック破損またはクロージャ実行時エラーを返します。
    pub fn with_graph_mut<R>(
        &self,
        writer: impl FnOnce(&mut CompiledGraph) -> AqueductResult<R>,
    ) -> AqueductResult<R> {
        let snapshot = self.snapshot();
        let mut guard = snapshot.lock().map_err(|_error| {
            AqueductError::new(
                ErrorKind::Runtime,
                "LIVE_GRAPH_LOCK_POISONED",
                "LiveGraph ロックが壊れています",
            )
        })?;
        writer(&mut guard)
    }

    /// 1 tick 実行する。
    ///
    /// # Errors
    /// 評価中に発生したランタイムエラーを返します。
    pub fn tick(&self, tick: u64) -> AqueductResult<()> {
        self.with_graph_mut(|graph| graph.tick(tick))
    }
}

/// Tick 駆動実行器。
pub struct TickDriver {
    live_graph: Arc<LiveGraph>,
    next_tick: u64,
}

impl TickDriver {
    /// ドライバを作成する。
    #[must_use]
    pub fn new(live_graph: Arc<LiveGraph>) -> Self {
        Self {
            live_graph,
            next_tick: 0,
        }
    }

    /// 次に実行される tick 値を返す。
    #[must_use]
    pub const fn next_tick(&self) -> u64 {
        self.next_tick
    }

    /// 1 tick 実行して実行済み tick 値を返す。
    ///
    /// # Errors
    /// 評価中に発生したランタイムエラーを返します。
    pub fn run_tick(&mut self) -> AqueductResult<u64> {
        let tick = self.next_tick;
        self.live_graph.tick(tick)?;
        self.next_tick = self.next_tick.saturating_add(1);
        Ok(tick)
    }
}
