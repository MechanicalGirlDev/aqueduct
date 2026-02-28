//! サーバー実行ループ。

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::Duration;

use aqueduct_core::{AqueductError, AqueductResult, ErrorKind, TickDriver};
use aqueduct_protocol::{ServerEnvelope, ServerMessage};
use tokio::sync::mpsc;
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;
use tracing::{error, warn};

use crate::dispatcher::MessageDispatcher;
use crate::session::SessionPinDiff;
use crate::transport::{TransportServer, TransportSession};

type SessionSenderMap = Arc<RwLock<HashMap<String, mpsc::UnboundedSender<ServerEnvelope>>>>;

const DEFAULT_TICK_INTERVAL: Duration = Duration::from_millis(16);

/// サーバー accept ループと各セッション処理を実行します。
///
/// # Errors
/// accept 失敗、tick 処理失敗、または内部ロック失敗時にエラーを返します。
pub async fn run_server(
    server: Arc<dyn TransportServer>,
    dispatcher: Arc<MessageDispatcher>,
    shutdown: CancellationToken,
) -> AqueductResult<()> {
    let session_senders: SessionSenderMap = Arc::new(RwLock::new(HashMap::new()));
    let mut session_tasks: JoinSet<AqueductResult<()>> = JoinSet::new();

    let tick_task = tokio::spawn(run_tick_loop(
        Arc::clone(&dispatcher),
        Arc::clone(&session_senders),
        shutdown.child_token(),
    ));

    loop {
        tokio::select! {
            () = shutdown.cancelled() => break,
            accept_result = server.accept() => {
                let session = accept_result?;
                spawn_session_task(
                    session,
                    &dispatcher,
                    &session_senders,
                    shutdown.child_token(),
                    &mut session_tasks,
                )?;
            }
            joined = session_tasks.join_next(), if !session_tasks.is_empty() => {
                if let Some(join_result) = joined {
                    handle_session_join_result(join_result);
                }
            }
        }
    }

    shutdown.cancel();

    match tick_task.await {
        Ok(Ok(())) => {}
        Ok(Err(error)) => return Err(error),
        Err(join_error) => {
            return Err(AqueductError::new(
                ErrorKind::Server,
                "SERVER_TICK_TASK_JOIN_FAILED",
                format!("tick タスクの join に失敗しました: {join_error}"),
            ));
        }
    }

    while let Some(join_result) = session_tasks.join_next().await {
        handle_session_join_result(join_result);
    }

    Ok(())
}

fn spawn_session_task(
    session: Box<dyn TransportSession>,
    dispatcher: &Arc<MessageDispatcher>,
    session_senders: &SessionSenderMap,
    shutdown: CancellationToken,
    task_set: &mut JoinSet<AqueductResult<()>>,
) -> AqueductResult<()> {
    let session_id = session.session_id().to_owned();
    dispatcher
        .session_manager()
        .add_session(session_id.clone())?;

    let (outbound_tx, outbound_rx) = mpsc::unbounded_channel();
    {
        let mut guard = session_senders.write().map_err(|_error| {
            AqueductError::new(
                ErrorKind::Server,
                "SERVER_SESSION_SENDER_WRITE_LOCK_POISONED",
                "セッション送信マップのロックが壊れています",
            )
        })?;
        let _ = guard.insert(session_id.clone(), outbound_tx);
    }

    let task_dispatcher = Arc::clone(dispatcher);
    let task_session_senders = Arc::clone(session_senders);
    task_set.spawn(async move {
        let loop_result = run_session_loop(
            session,
            session_id.clone(),
            Arc::clone(&task_dispatcher),
            outbound_rx,
            shutdown,
        )
        .await;

        if let Err(error) = task_dispatcher
            .session_manager()
            .remove_session(&session_id)
        {
            warn!(
                session_id = %session_id,
                "セッション削除に失敗しました: {}",
                error
            );
        }

        if let Err(error) = remove_session_sender(&task_session_senders, &session_id) {
            warn!(
                session_id = %session_id,
                "セッション送信エントリ削除に失敗しました: {}",
                error
            );
        }

        loop_result
    });

    Ok(())
}

async fn run_session_loop(
    mut session: Box<dyn TransportSession>,
    session_id: String,
    dispatcher: Arc<MessageDispatcher>,
    mut outbound_rx: mpsc::UnboundedReceiver<ServerEnvelope>,
    shutdown: CancellationToken,
) -> AqueductResult<()> {
    loop {
        tokio::select! {
            () = shutdown.cancelled() => break,
            outbound = outbound_rx.recv() => {
                let Some(outbound) = outbound else {
                    break;
                };
                session.send(outbound).await?;
            }
            inbound = session.recv() => {
                let Some(inbound) = inbound? else {
                    break;
                };

                if let Some(response) = dispatcher.dispatch(&session_id, inbound)? {
                    session.send(response).await?;
                }
            }
        }
    }

    Ok(())
}

async fn run_tick_loop(
    dispatcher: Arc<MessageDispatcher>,
    session_senders: SessionSenderMap,
    shutdown: CancellationToken,
) -> AqueductResult<()> {
    let mut tick_driver = TickDriver::new(dispatcher.live_graph());
    let mut interval = tokio::time::interval(DEFAULT_TICK_INTERVAL);

    loop {
        tokio::select! {
            () = shutdown.cancelled() => break,
            _ = interval.tick() => {
                if !dispatcher.is_runtime_running() {
                    continue;
                }

                if let Err(error) = tick_driver.run_tick() {
                    dispatcher.mark_runtime_error();
                    error!("tick 実行に失敗しました: {}", error);
                    continue;
                }

                let pin_store = dispatcher
                    .live_graph()
                    .with_graph(|graph| Ok(graph.pin_store_snapshot()))?;
                let pin_diffs = dispatcher.session_manager().collect_pin_diffs(&pin_store)?;
                let graph_rev = dispatcher.current_graph_rev()?;

                for pin_diff in pin_diffs {
                    send_pin_diff(&session_senders, pin_diff, graph_rev)?;
                }
            }
        }
    }

    Ok(())
}

fn send_pin_diff(
    session_senders: &SessionSenderMap,
    pin_diff: SessionPinDiff,
    graph_rev: u64,
) -> AqueductResult<()> {
    let envelope = ServerEnvelope {
        request_id: None,
        body: ServerMessage::PinValues {
            values: pin_diff.values,
        },
        graph_rev,
    };

    let maybe_sender = {
        let guard = session_senders.read().map_err(|_error| {
            AqueductError::new(
                ErrorKind::Server,
                "SERVER_SESSION_SENDER_READ_LOCK_POISONED",
                "セッション送信マップのロックが壊れています",
            )
        })?;
        guard.get(&pin_diff.session_id).cloned()
    };

    if let Some(sender) = maybe_sender {
        if sender.send(envelope).is_err() {
            warn!(session_id = %pin_diff.session_id, "切断済みセッションへの送信をスキップしました");
        }
    }

    Ok(())
}

fn remove_session_sender(
    session_senders: &SessionSenderMap,
    session_id: &str,
) -> AqueductResult<()> {
    let mut guard = session_senders.write().map_err(|_error| {
        AqueductError::new(
            ErrorKind::Server,
            "SERVER_SESSION_SENDER_WRITE_LOCK_POISONED",
            "セッション送信マップのロックが壊れています",
        )
    })?;
    let _ = guard.remove(session_id);
    Ok(())
}

fn handle_session_join_result(join_result: Result<AqueductResult<()>, tokio::task::JoinError>) {
    match join_result {
        Ok(Ok(())) => {}
        Ok(Err(error)) => warn!("セッションループがエラー終了しました: {}", error),
        Err(join_error) => warn!("セッションタスクの join に失敗しました: {}", join_error),
    }
}
