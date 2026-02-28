#![allow(clippy::too_many_lines)]

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};

use aqueduct_core::{
    AqueductError, AqueductResult, ErrorKind, GraphCompiler, LiveGraph, NodeRegistry,
};
use aqueduct_nodes::register_all;
use aqueduct_protocol::{
    ClientEnvelope, ClientMessage, Graph, GraphMutation, NodeId, NodeInstance, RuntimeState,
    ServerEnvelope, ServerMessage, PROTOCOL_VERSION,
};
use aqueduct_server::{MessageDispatcher, SessionManager, TransportSession};

struct MockTransportSession {
    session_id: String,
    inbound: VecDeque<ClientEnvelope>,
    outbound: Mutex<VecDeque<ServerEnvelope>>,
}

impl MockTransportSession {
    fn new(session_id: impl Into<String>, inbound: VecDeque<ClientEnvelope>) -> Self {
        Self {
            session_id: session_id.into(),
            inbound,
            outbound: Mutex::new(VecDeque::new()),
        }
    }

    fn sent_messages(&self) -> AqueductResult<Vec<ServerEnvelope>> {
        let guard = self.outbound.lock().map_err(|_error| {
            AqueductError::new(
                ErrorKind::Server,
                "TEST_MOCK_SESSION_OUTBOUND_LOCK_POISONED",
                "MockTransportSession の outbound ロックが壊れています",
            )
        })?;
        Ok(guard.iter().cloned().collect())
    }
}

#[async_trait::async_trait]
impl TransportSession for MockTransportSession {
    async fn recv(&mut self) -> AqueductResult<Option<ClientEnvelope>> {
        Ok(self.inbound.pop_front())
    }

    async fn send(&self, msg: ServerEnvelope) -> AqueductResult<()> {
        let mut guard = self.outbound.lock().map_err(|_error| {
            AqueductError::new(
                ErrorKind::Server,
                "TEST_MOCK_SESSION_OUTBOUND_LOCK_POISONED",
                "MockTransportSession の outbound ロックが壊れています",
            )
        })?;
        guard.push_back(msg);
        Ok(())
    }

    fn session_id(&self) -> &str {
        &self.session_id
    }
}

#[test]
fn test_session_manager_subscribe_unsubscribe() {
    let manager = SessionManager::new();
    must_ok(manager.add_session("session-a"));

    must_ok(manager.subscribe_pins(
        "session-a",
        &[
            aqueduct_protocol::PinId::from("out"),
            aqueduct_protocol::PinId::from("tick"),
        ],
    ));
    let subscribed = must_some(must_ok(manager.subscribed_pins("session-a")));
    assert!(subscribed
        .iter()
        .any(|scoped| scoped.pin_id == aqueduct_protocol::PinId::from("out")));
    assert!(subscribed
        .iter()
        .any(|scoped| scoped.pin_id == aqueduct_protocol::PinId::from("tick")));

    must_ok(manager.unsubscribe_pins("session-a", &[aqueduct_protocol::PinId::from("out")]));
    let subscribed = must_some(must_ok(manager.subscribed_pins("session-a")));
    assert!(!subscribed
        .iter()
        .any(|scoped| scoped.pin_id == aqueduct_protocol::PinId::from("out")));
    assert!(subscribed
        .iter()
        .any(|scoped| scoped.pin_id == aqueduct_protocol::PinId::from("tick")));

    must_ok(manager.remove_session("session-a"));
    assert!(must_ok(manager.subscribed_pins("session-a")).is_none());
}

#[tokio::test(flavor = "current_thread")]
async fn test_dispatcher_handshake() {
    let (dispatcher, session_manager) = must_ok(build_dispatcher());
    let inbound = VecDeque::from([ClientEnvelope {
        request_id: 1,
        body: ClientMessage::Handshake {
            protocol_version: PROTOCOL_VERSION.to_owned(),
        },
    }]);
    let mut session = MockTransportSession::new("session-hs", inbound);
    must_ok(session_manager.add_session(session.session_id().to_owned()));

    must_ok(pump_session(&dispatcher, &mut session).await);
    let sent = must_ok(session.sent_messages());
    assert_eq!(sent.len(), 1);

    let response = &sent[0];
    assert_eq!(response.request_id, Some(1));
    assert_eq!(response.graph_rev, 0);
    assert_eq!(
        response.body,
        ServerMessage::Handshake {
            protocol_version: PROTOCOL_VERSION.to_owned()
        }
    );
}

#[tokio::test(flavor = "current_thread")]
async fn test_dispatcher_registry_list() {
    let (dispatcher, session_manager) = must_ok(build_dispatcher());
    must_ok(session_manager.add_session("session-reg"));

    let response = must_some(must_ok(dispatcher.dispatch(
        "session-reg",
        ClientEnvelope {
            request_id: 2,
            body: ClientMessage::RegistryList,
        },
    )));
    assert_eq!(response.request_id, Some(2));

    let ServerMessage::RegistryNodes { defs } = response.body else {
        panic!("registry.nodes 以外のメッセージを受信しました");
    };
    assert!(
        defs.iter().any(|def| def.type_name == "math.add"),
        "registry.list の結果に `math.add` が含まれていません"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn test_dispatcher_graph_load_and_compile() {
    let (dispatcher, session_manager) = must_ok(build_dispatcher());
    must_ok(session_manager.add_session("session-graph"));

    let graph = graph_with_nodes(vec![node_instance("adder", "math.add")]);
    let loaded = must_some(must_ok(dispatcher.dispatch(
        "session-graph",
        ClientEnvelope {
            request_id: 10,
            body: ClientMessage::GraphLoad {
                graph: graph.clone(),
            },
        },
    )));
    assert_eq!(loaded.request_id, Some(10));
    assert_eq!(loaded.graph_rev, 1);

    let ServerMessage::GraphCompiled {
        eval_order,
        warnings,
    } = loaded.body
    else {
        panic!("graph.compiled 以外のメッセージを受信しました");
    };
    assert_eq!(warnings, Vec::<String>::new());
    assert_eq!(eval_order, vec![NodeId::from("adder")]);

    let compiled = must_some(must_ok(dispatcher.dispatch(
        "session-graph",
        ClientEnvelope {
            request_id: 11,
            body: ClientMessage::GraphCompile,
        },
    )));
    assert_eq!(compiled.request_id, Some(11));
    assert_eq!(compiled.graph_rev, 2);

    let ServerMessage::GraphCompiled {
        eval_order,
        warnings,
    } = compiled.body
    else {
        panic!("graph.compiled 以外のメッセージを受信しました");
    };
    assert_eq!(warnings, Vec::<String>::new());
    assert_eq!(eval_order, vec![NodeId::from("adder")]);
}

#[tokio::test(flavor = "current_thread")]
async fn test_dispatcher_graph_mutate() {
    let (dispatcher, session_manager) = must_ok(build_dispatcher());
    must_ok(session_manager.add_session("session-mutate"));

    let initial = graph_with_nodes(vec![node_instance("adder", "math.add")]);
    let _ = must_some(must_ok(dispatcher.dispatch(
        "session-mutate",
        ClientEnvelope {
            request_id: 20,
            body: ClientMessage::GraphLoad { graph: initial },
        },
    )));

    let mutated = must_some(must_ok(dispatcher.dispatch(
        "session-mutate",
        ClientEnvelope {
            request_id: 21,
            body: ClientMessage::GraphMutate {
                mutations: vec![GraphMutation::AddNode {
                    instance: node_instance("clock", "time.tick"),
                }],
            },
        },
    )));
    assert_eq!(mutated.request_id, Some(21));
    assert_eq!(mutated.graph_rev, 2);

    let ServerMessage::GraphCompiled {
        eval_order,
        warnings,
    } = mutated.body
    else {
        panic!("graph.compiled 以外のメッセージを受信しました");
    };
    assert_eq!(warnings, Vec::<String>::new());
    assert_eq!(eval_order.len(), 2);
    assert!(eval_order.contains(&NodeId::from("adder")));
    assert!(eval_order.contains(&NodeId::from("clock")));
}

#[tokio::test(flavor = "current_thread")]
async fn test_dispatcher_runtime_start_stop() {
    let (dispatcher, session_manager) = must_ok(build_dispatcher());
    must_ok(session_manager.add_session("session-runtime"));

    let started = must_some(must_ok(dispatcher.dispatch(
        "session-runtime",
        ClientEnvelope {
            request_id: 30,
            body: ClientMessage::RuntimeStart,
        },
    )));
    assert_eq!(started.request_id, Some(30));
    assert_eq!(
        started.body,
        ServerMessage::RuntimeState {
            state: RuntimeState::Running
        }
    );
    assert!(dispatcher.is_runtime_running());

    let stopped = must_some(must_ok(dispatcher.dispatch(
        "session-runtime",
        ClientEnvelope {
            request_id: 31,
            body: ClientMessage::RuntimeStop,
        },
    )));
    assert_eq!(stopped.request_id, Some(31));
    assert_eq!(
        stopped.body,
        ServerMessage::RuntimeState {
            state: RuntimeState::Stopped
        }
    );
    assert!(!dispatcher.is_runtime_running());
}

async fn pump_session(
    dispatcher: &MessageDispatcher,
    session: &mut MockTransportSession,
) -> AqueductResult<()> {
    loop {
        let inbound = session.recv().await?;
        let Some(inbound) = inbound else {
            break;
        };

        if let Some(response) = dispatcher.dispatch(session.session_id(), inbound)? {
            session.send(response).await?;
        }
    }

    Ok(())
}

fn build_dispatcher() -> AqueductResult<(Arc<MessageDispatcher>, Arc<SessionManager>)> {
    let mut registry = NodeRegistry::new();
    register_all(&mut registry)?;
    let registry = Arc::new(registry);

    let compiler = GraphCompiler::new(registry.as_ref());
    let compiled = compiler.compile(&Graph::default(), 0)?;
    let live_graph = Arc::new(LiveGraph::new(Arc::new(Mutex::new(compiled))));
    let session_manager = Arc::new(SessionManager::new());
    let dispatcher = Arc::new(MessageDispatcher::new(
        live_graph,
        Arc::clone(&registry),
        Arc::clone(&session_manager),
    ));

    Ok((dispatcher, session_manager))
}

fn node_instance(id: &str, type_name: &str) -> NodeInstance {
    NodeInstance {
        id: NodeId::from(id),
        type_name: type_name.to_owned(),
        properties: HashMap::new(),
        position: (0.0, 0.0),
    }
}

fn graph_with_nodes(nodes: Vec<NodeInstance>) -> Graph {
    let nodes = nodes
        .into_iter()
        .map(|instance| (instance.id.clone(), instance))
        .collect();
    Graph {
        nodes,
        edges: Vec::new(),
    }
}

fn must_ok<T>(result: AqueductResult<T>) -> T {
    match result {
        Ok(value) => value,
        Err(error) => panic!("unexpected error: {error}"),
    }
}

fn must_some<T>(value: Option<T>) -> T {
    match value {
        Some(value) => value,
        None => panic!("expected Some(..), got None"),
    }
}
