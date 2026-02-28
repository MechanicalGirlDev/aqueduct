//! セッション管理。

use std::collections::{HashMap, HashSet};
use std::sync::RwLock;

use aqueduct_core::{AqueductError, AqueductResult, ErrorKind, PinStore, ScopedPinId};
use aqueduct_protocol::{NodeId, PinId, PinValue};

const WILDCARD_NODE_ID: &str = "__all_nodes__";

/// セッションごとの差分送信用状態。
#[derive(Debug, Clone, Default)]
pub struct SessionState {
    subscribed_pins: HashSet<ScopedPinId>,
    last_sent_values: HashMap<PinId, PinValue>,
}

impl SessionState {
    fn is_subscribed_to_pin_id(&self, pin_id: &PinId) -> bool {
        self.subscribed_pins
            .iter()
            .any(|scoped_pin| scoped_pin.pin_id == *pin_id)
    }
}

/// セッションごとのピン値差分。
#[derive(Debug, Clone, PartialEq)]
pub struct SessionPinDiff {
    /// 対象セッション ID。
    pub session_id: String,
    /// 変化した `PinValue` マップ。
    pub values: HashMap<PinId, PinValue>,
}

/// セッション状態管理。
#[derive(Debug, Default)]
pub struct SessionManager {
    sessions: RwLock<HashMap<String, SessionState>>,
}

impl SessionManager {
    /// 空の `SessionManager` を作成します。
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// セッションを追加します。
    ///
    /// 既存 ID がある場合は状態を初期化して上書きします。
    ///
    /// # Errors
    /// 内部ロックに失敗した場合にエラーを返します。
    pub fn add_session(&self, session_id: impl Into<String>) -> AqueductResult<()> {
        let mut guard = self
            .sessions
            .write()
            .map_err(|_error| lock_poisoned("SESSION_MANAGER_ADD_LOCK_POISONED"))?;
        let _ = guard.insert(session_id.into(), SessionState::default());
        Ok(())
    }

    /// セッションを削除します。
    ///
    /// # Errors
    /// 内部ロックに失敗した場合にエラーを返します。
    pub fn remove_session(&self, session_id: &str) -> AqueductResult<()> {
        let mut guard = self
            .sessions
            .write()
            .map_err(|_error| lock_poisoned("SESSION_MANAGER_REMOVE_LOCK_POISONED"))?;
        let _ = guard.remove(session_id);
        Ok(())
    }

    /// セッションの購読ピンを追加します。
    ///
    /// `PinId` はノード非依存で扱われるため、内部ではワイルドカード `NodeId` で保持します。
    ///
    /// # Errors
    /// セッション未登録または内部ロック失敗時にエラーを返します。
    pub fn subscribe_pins(&self, session_id: &str, pin_ids: &[PinId]) -> AqueductResult<()> {
        let mut guard = self
            .sessions
            .write()
            .map_err(|_error| lock_poisoned("SESSION_MANAGER_SUBSCRIBE_LOCK_POISONED"))?;
        let Some(state) = guard.get_mut(session_id) else {
            return Err(AqueductError::new(
                ErrorKind::Server,
                "SESSION_MANAGER_SESSION_NOT_FOUND",
                format!("セッションが見つかりません: {session_id}"),
            ));
        };

        for pin_id in pin_ids {
            let _ = state
                .subscribed_pins
                .insert(wildcard_scoped_pin(pin_id.clone()));
        }

        Ok(())
    }

    /// セッションの購読ピンを削除します。
    ///
    /// # Errors
    /// セッション未登録または内部ロック失敗時にエラーを返します。
    pub fn unsubscribe_pins(&self, session_id: &str, pin_ids: &[PinId]) -> AqueductResult<()> {
        let mut guard = self
            .sessions
            .write()
            .map_err(|_error| lock_poisoned("SESSION_MANAGER_UNSUBSCRIBE_LOCK_POISONED"))?;
        let Some(state) = guard.get_mut(session_id) else {
            return Err(AqueductError::new(
                ErrorKind::Server,
                "SESSION_MANAGER_SESSION_NOT_FOUND",
                format!("セッションが見つかりません: {session_id}"),
            ));
        };

        let target_pin_ids: HashSet<PinId> = pin_ids.iter().cloned().collect();
        state
            .subscribed_pins
            .retain(|scoped_pin| !target_pin_ids.contains(&scoped_pin.pin_id));
        state
            .last_sent_values
            .retain(|pin_id, _value| !target_pin_ids.contains(pin_id));

        Ok(())
    }

    /// セッションの購読ピン集合を取得します。
    ///
    /// # Errors
    /// 内部ロックに失敗した場合にエラーを返します。
    pub fn subscribed_pins(
        &self,
        session_id: &str,
    ) -> AqueductResult<Option<HashSet<ScopedPinId>>> {
        let guard = self
            .sessions
            .read()
            .map_err(|_error| lock_poisoned("SESSION_MANAGER_READ_LOCK_POISONED"))?;
        Ok(guard
            .get(session_id)
            .map(|state| state.subscribed_pins.clone()))
    }

    /// 現在の `PinStore` から、各セッションの未送信差分を抽出します。
    ///
    /// # Errors
    /// 内部ロックに失敗した場合にエラーを返します。
    pub fn collect_pin_diffs(&self, pin_store: &PinStore) -> AqueductResult<Vec<SessionPinDiff>> {
        let mut guard = self
            .sessions
            .write()
            .map_err(|_error| lock_poisoned("SESSION_MANAGER_DIFF_LOCK_POISONED"))?;
        let mut session_diffs = Vec::new();

        for (session_id, state) in guard.iter_mut() {
            let mut changed_values = HashMap::new();
            for (scoped_pin_id, value) in pin_store.value_entries() {
                if !state.is_subscribed_to_pin_id(&scoped_pin_id.pin_id) {
                    continue;
                }

                if state.last_sent_values.get(&scoped_pin_id.pin_id) == Some(value) {
                    continue;
                }

                let _ = changed_values.insert(scoped_pin_id.pin_id.clone(), value.clone());
                let _ = state
                    .last_sent_values
                    .insert(scoped_pin_id.pin_id.clone(), value.clone());
            }

            if !changed_values.is_empty() {
                session_diffs.push(SessionPinDiff {
                    session_id: session_id.clone(),
                    values: changed_values,
                });
            }
        }

        Ok(session_diffs)
    }
}

fn wildcard_scoped_pin(pin_id: PinId) -> ScopedPinId {
    ScopedPinId::new(NodeId::from(WILDCARD_NODE_ID), pin_id)
}

fn lock_poisoned(code: &'static str) -> AqueductError {
    AqueductError::new(
        ErrorKind::Server,
        code,
        "SessionManager のロックが壊れています",
    )
}
