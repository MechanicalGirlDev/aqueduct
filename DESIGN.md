# Node-Based Dataflow Engine Design

## 概要

vvvv にインスパイアされたリアルタイムノードベースデータフローエンジン。GUI のノード操作からデータフローパイプラインを構築し、character-engine のコンポーネント群と連携する。

## 要件

- リアルタイム連続処理（tick ごとにデータがノードグラフ全体を流れる）
- ホットパッチ（実行中にノードの接続を変更しても止まらない）
- イベント駆動（character-engine の既存イベントバスをトリガーとして利用可能）
- プラグインによるノード拡張（Rust / WASM / character-engine ブリッジ）
- フロントエンド: Web ベース（React + React Flow）、将来 Tauri 拡張可能
- character-engine をライブラリとして依存するが、フレームワークとしては独立

## リポジトリ分離方針

### 別リポジトリ: `aqueduct`（コアフレームワーク）

vvvv 的なコア機能。character-engine に依存しない汎用データフローエンジン。
リポジトリ: `/home/nop/dev/mechanicalgirl/aqueduct/`

### このリポジトリ: `character-engine`（ドメイン統合）

コアフレームワークを利用したブリッジノード群と統合バイナリ。

### 互換ポリシー

- `aqueduct-protocol` クレートは semver 厳守。breaking change は major bump
- WebSocket 接続時に `protocol_version` ハンドシェイクを行い、互換性を検証
- `ce-node-graph` は `aqueduct-core` の特定 semver 範囲に依存

## アーキテクチャ

### 評価モデル: 同期 DAG 評価 + 非同期ジョブ補助

vvvv gamma と同様のコンパイル方式。グラフ定義からコンパイル済みの評価スケジュールを生成する。

**実行モデル**: TickDriver が毎 tick、トポロジカル順にノードの `evaluate()` を**同期的に逐次呼び出す**。
非同期処理（API呼び出し等）はランタイム側の `JoinSet` で管理し、ノード自体はブロックしない。

```
Graph (JSON)
  | validate（型チェック、循環検出）
  | topological sort
  | compile
  v
CompiledGraph
  ├── eval_order: Vec<NodeId>          -- トポロジカル順の評価スケジュール
  ├── evaluators: Vec<Box<dyn NodeEvaluator>>  -- 各ノードの評価関数
  ├── pin_store: PinStore              -- 全ピン値のスナップショット
  └── async_jobs: JoinSet<AsyncJobResult>      -- 非同期ジョブ管理
```

### グラフデータモデル

```rust
/// ピンの型（ノード間を流れるデータの型）
enum PinType {
    Float, Int, Bool, String, Json,
    Event,  // トリガー信号（データなし）
    Any,    // 動的型（接続時に解決）
}

/// ピン定義（ノードの入出力ポート）
struct PinDef {
    id: PinId,
    name: String,
    pin_type: PinType,
    direction: Direction,  // Input | Output
}

/// ノード定義（ノードの「型」を表す）
struct NodeDef {
    type_name: String,         // e.g. "math.add", "discord.send"
    inputs: Vec<PinDef>,
    outputs: Vec<PinDef>,
    properties: Vec<Property>, // GUI で編集可能なパラメータ
}

/// グラフ上に配置されたノードインスタンス
struct NodeInstance {
    id: NodeId,
    type_name: String,
    properties: HashMap<String, serde_json::Value>,
    position: (f32, f32),  // GUI 用座標
}

/// エッジ（ピン間の接続）
struct Edge {
    id: EdgeId,
    from_node: NodeId, from_pin: PinId,
    to_node: NodeId,   to_pin: PinId,
}

/// グラフ全体
struct Graph {
    nodes: HashMap<NodeId, NodeInstance>,
    edges: Vec<Edge>,
}
```

### ピン値ストア（チャネルレス設計）

`tokio::watch` チャネルではなく、**中央集権的なピン値ストア**を使用する。

理由:
- `watch` は Event ピン（パルス信号）と相性が悪い（最新値のみ保持でパルスが潰れる）
- tick 一貫スナップショットの保証が容易
- TickDriver が同期評価するため、チャネルベースの通信は不要

```rust
/// 全ピンの値を保持するストア
struct PinStore {
    /// 値ピン: 最新値を保持（Float, Int, Bool, String, Json, Any）
    values: HashMap<PinId, PinValue>,
    /// Event ピン: tick 内のトリガーフラグ（tick 終了時にクリア）
    events: HashSet<PinId>,
}

impl PinStore {
    /// 値ピンの読み取り
    fn get_value(&self, pin_id: PinId) -> Option<&PinValue>;
    /// Event ピンのトリガー状態を確認
    fn is_event_fired(&self, pin_id: PinId) -> bool;
    /// 値ピンの書き込み
    fn set_value(&mut self, pin_id: PinId, value: PinValue);
    /// Event ピンを発火
    fn fire_event(&mut self, pin_id: PinId);
    /// tick 終了時にイベントフラグをクリア
    fn clear_events(&mut self);
}
```

### コンパイルパイプライン

```rust
// 1. トポロジカルソートで評価順序を確定
let eval_order: Vec<NodeId> = toposort(&graph)?;

// 2. PinStore を初期化（全ピンにデフォルト値を設定）
let pin_store = PinStore::new(&graph);

// 3. 各ノードの評価関数を生成
let mut evaluators: Vec<(NodeId, Box<dyn NodeEvaluator>)> = Vec::new();
for node_id in &eval_order {
    let node = &graph.nodes[node_id];
    let factory = registry.get(&node.type_name)?;
    let evaluator = factory.create(node.properties.clone())?;
    evaluators.push((*node_id, evaluator));
}

// 4. エッジからピン接続マップを構築
//    input_pin → source_output_pin のマッピング
let connections: HashMap<PinId, PinId> = build_connection_map(&graph.edges);
```

### Tick 駆動モデル

```rust
// TickDriver が毎 tick、トポロジカル順にノードを同期評価
loop {
    tokio::select! {
        _ = tick_interval.tick() => {
            let graph = live_graph.current.load();
            let tick = tick_counter.fetch_add(1, Ordering::SeqCst);

            // 1. 完了した非同期ジョブの結果を PinStore に反映
            graph.drain_completed_jobs();

            // 2. トポロジカル順に全ノードを評価
            for (node_id, evaluator) in &mut graph.evaluators {
                // 入力ピンの値を接続マップ経由で収集
                let inputs = graph.collect_inputs(node_id);
                match evaluator.evaluate(&inputs, tick)? {
                    NodeEvalResult::Ready(outputs) => {
                        graph.pin_store.write_outputs(node_id, outputs);
                    }
                    NodeEvalResult::Spawned { job_id } => {
                        // ランタイムの JoinSet で追跡（世代番号付き）
                    }
                }
            }

            // 3. tick 終了: Event フラグをクリア
            graph.pin_store.clear_events();
        }
        _ = shutdown.cancelled() => break,
    }
}
```

### 非同期ノード

API 呼び出しや LLM 推論のような重い処理への対応。

```rust
enum NodeEvalResult {
    /// 即座に出力（同期ノード）
    Ready(Vec<PinValue>),
    /// 非同期ジョブを発火（ランタイム側で JoinSet 管理）
    Spawned { job_id: u64 },
}

/// 非同期ジョブの完了結果
struct AsyncJobResult {
    job_id: u64,
    node_id: NodeId,
    tick_spawned: u64,      // 発火した tick（世代管理）
    outputs: Vec<PinValue>,
}
```

世代管理: ホットパッチでノードが削除/再作成された場合、古い `job_id` の結果は `tick_spawned` と現在のノード世代を比較して破棄する。

### 主要な設計判断

| 項目 | 決定 | 理由 |
|------|------|------|
| データ伝搬 | PinStore（中央集権） | tick 一貫スナップショット保証。Event ピンのパルス管理が容易 |
| 評価モデル | 同期 DAG 評価 | TickDriver が逐次呼び出し。決定性が保証される |
| 非同期ノード | Spawned + JoinSet + 世代管理 | ブロックしない。古い結果の混入を防止 |

## ホットパッチ

### 差分操作

```rust
enum GraphMutation {
    AddNode { instance: NodeInstance },
    RemoveNode { id: NodeId },
    AddEdge { edge: Edge },
    RemoveEdge { id: EdgeId },
    UpdateProperty { node_id: NodeId, key: String, value: serde_json::Value },
}
```

### パッチ戦略: ArcSwap によるスナップショット差し替え

`RwLock` ではなく `arc_swap::ArcSwap` でアトミックにグラフを差し替える。
これにより tick 評価中にパッチがブロックされない。

```rust
struct LiveGraph {
    /// 現在のコンパイル済みグラフ（ArcSwap でアトミック差し替え）
    current: arc_swap::ArcSwap<CompiledGraph>,
    /// パッチ要求チャネル
    patch_tx: mpsc::Sender<Vec<GraphMutation>>,
}
```

### パッチ適用フロー

```
GraphMutation 受信
  | バリデーション（型チェック、循環検出）
  | 影響分析: 変更されたノードの下流（依存先）を特定
  | affected_nodes = downstream_of(changed_nodes)
  v
現在の CompiledGraph をベースに差分リコンパイル
  | 新しい CompiledGraph を生成（別タスクで実行可能）
  | 未変更ノードの evaluator と PinStore の値を引き継ぎ
  v
ArcSwap::store() でアトミックに差し替え
  → 次の tick から新グラフで評価開始
  → 現 tick は旧グラフで最後まで完走（一貫性保証）
```

### UpdateProperty の最適化

`UpdateProperty` はリコンパイル不要。evaluator の `apply_property_patch()` を直接呼ぶ。
パッチチャネル経由で tick 間に適用。

### 安全性の保証

| リスク | 対策 |
|--------|------|
| tick 中のグラフ変更 | ArcSwap で次 tick から反映。現 tick は旧スナップショットで完走 |
| 削除ノードの非同期ジョブ | 世代管理（tick_spawned）で古いジョブ結果を破棄 |
| 型不一致の接続 | パッチ適用前にバリデーション。不正な接続は reject してクライアントにエラー返却 |
| パッチ中の PinStore 値喪失 | 未変更ピンの値は新 CompiledGraph に引き継ぎ |

## プラグインシステム

### NodeFactory trait

```rust
trait NodeFactory: Send + Sync {
    /// ノード定義（ピン、プロパティの宣言）
    fn node_def(&self) -> NodeDef;

    /// プロパティから評価関数を生成
    fn create(&self, properties: HashMap<String, serde_json::Value>)
        -> AppResult<Box<dyn NodeEvaluator>>;
}

/// tick ごとに呼ばれる評価関数
trait NodeEvaluator: Send {
    /// 同期評価（&mut self で内部状態を持てる）
    fn evaluate(&mut self, inputs: &[PinValue], tick: u64) -> AppResult<NodeEvalResult>;

    /// プロパティの動的更新（リコンパイル不要）
    fn apply_property_patch(&mut self, key: &str, value: &serde_json::Value) -> AppResult<()>;
}
```

### 3層プラグインモデル

| Layer | 説明 | 実行方式 |
|-------|------|----------|
| Layer 1: ビルトイン | math, string, logic, time, convert | Rust 静的リンク |
| Layer 2: WASM | ユーザー拡張ノード | wasmtime + WIT (Component Model) |
| Layer 3: ブリッジ | character-engine コンポーネントラップ | Rust, ComponentContainer から DI |

### WASM プラグイン ABI (WIT)

`wasm_bindgen + JSON文字列` ではなく、WIT (WebAssembly Interface Types / Component Model) を使用。
型安全性と性能を確保し、wasmtime ホストとの整合性を保つ。

```wit
// node-plugin.wit
package aqueduct:plugin@0.1.0;

interface types {
    variant pin-value {
        float(f64),
        int(s64),
        bool(bool),
        text(string),
        json(string),
        event,
        none,
    }

    record pin-def {
        id: string,
        name: string,
        pin-type: string,
        direction: string,
    }

    record node-def {
        type-name: string,
        inputs: list<pin-def>,
        outputs: list<pin-def>,
    }

    record eval-result {
        outputs: list<pin-value>,
        is-async: bool,
    }
}

world node-plugin {
    use types.{pin-value, node-def, eval-result};

    export node-def: func() -> node-def;
    export evaluate: func(inputs: list<pin-value>, tick: u64) -> eval-result;
    export apply-property: func(key: string, value: string) -> result<_, string>;
}
```

```rust
// ホスト側: wasmtime + wit-bindgen でロード
struct WasmNodeFactory {
    component: wasmtime::component::Component,
    engine: wasmtime::Engine,
}
impl NodeFactory for WasmNodeFactory { /* WIT 経由で呼び出し */ }
```

### NodeRegistry

```rust
struct NodeRegistry {
    factories: HashMap<String, Arc<dyn NodeFactory>>,
}

impl NodeRegistry {
    fn register(&mut self, factory: Arc<dyn NodeFactory>);
    fn load_wasm_plugin(&mut self, path: &Path) -> AppResult<()>;
    fn load_builtin_nodes(&mut self);
    fn get(&self, type_name: &str) -> Option<Arc<dyn NodeFactory>>;
    fn list_node_defs(&self) -> Vec<NodeDef>;
}
```

## 通信層（Tauri 拡張対応）

### トランスポート抽象化

```
React (React Flow)
  |
  ├── TransportAdapter (抽象)
  |     ├── WebSocketTransport   ... ブラウザ向け
  |     └── TauriTransport       ... 将来追加
  |
  v  同一の Message プロトコル（Envelope 方式）
  |
Rust バックエンド
  |
  ├── TransportServer (抽象)
  |     ├── AxumWsServer         ... axum WebSocket
  |     └── TauriCommandServer   ... 将来追加
  |
  ├── GraphManager
  └── NodeRegistry
```

### Rust 側: セッション分離型 Transport

```rust
/// 接続を受け入れるサーバー
#[async_trait]
trait TransportServer: Send + Sync {
    /// 新しいクライアント接続を受け入れる
    async fn accept(&self) -> AppResult<Box<dyn TransportSession>>;
}

/// 個別クライアントセッション
#[async_trait]
trait TransportSession: Send + Sync {
    /// クライアントからのメッセージを受信
    async fn recv(&mut self) -> AppResult<Option<ClientEnvelope>>;
    /// クライアントへメッセージを送信
    async fn send(&self, msg: ServerEnvelope) -> AppResult<()>;
    /// セッション ID
    fn session_id(&self) -> &str;
}

/// クライアントからのメッセージ（相関ID + グラフリビジョン付き）
#[derive(Serialize, Deserialize)]
struct ClientEnvelope {
    request_id: u64,
    body: ClientMessage,
}

/// サーバーからのメッセージ
#[derive(Serialize, Deserialize)]
struct ServerEnvelope {
    /// リクエストへの応答の場合に設定
    request_id: Option<u64>,
    body: ServerMessage,
    /// 現在のグラフリビジョン（クライアント側での楽観的同期に使用）
    graph_rev: u64,
}
```

### フロントエンド側

```typescript
interface Transport {
  send(msg: ClientEnvelope): void;
  onMessage(handler: (msg: ServerEnvelope) => void): void;
  connect(): Promise<void>;
  disconnect(): void;
}

function createTransport(): Transport {
  if (window.__TAURI__) return new TauriTransport();
  return new WebSocketTransport('ws://localhost:PORT');
}
```

### プロトコル

```typescript
// Client -> Server (ClientEnvelope.body)
{ type: "graph.mutate", mutations: GraphMutation[] }
{ type: "graph.load", graph: Graph }
{ type: "graph.save" }
{ type: "graph.compile" }
{ type: "runtime.start" }
{ type: "runtime.stop" }
{ type: "runtime.set_tick_rate", hz: number }
{ type: "registry.list" }
{ type: "pin.subscribe", pin_ids: PinId[] }
{ type: "pin.unsubscribe", pin_ids: PinId[] }

// Server -> Client (ServerEnvelope.body)
{ type: "pin.values", values: { [pinId]: PinValue } }
{ type: "runtime.state", state: "running" | "stopped" | "error" }
{ type: "graph.compiled", eval_order: NodeId[], warnings: string[] }
{ type: "error", code: string, message: string, node_id?: NodeId }
{ type: "registry.nodes", defs: NodeDef[] }

// 接続時ハンドシェイク
{ type: "handshake", protocol_version: "0.1.0" }
```

### ピン値サブスクリプション

帯域節約のため、フロントエンドが表示中のピンのみ購読。変化があったピンだけ送信。
購読状態はセッション単位で管理（多クライアント対応）。

## クレート構成

### 別リポジトリ: `aqueduct`

```
aqueduct/
├── crates/
|   ├── aqueduct-core/          # Graph, Compiler, TickDriver, LiveGraph, Patch, PinStore
|   ├── aqueduct-protocol/      # ClientEnvelope, ServerEnvelope, ClientMessage, ServerMessage, PinValue
|   ├── aqueduct-nodes/         # ビルトインノード (math, string, logic, time, convert)
|   ├── aqueduct-wasm-host/     # wasmtime + WIT プラグインホスト
|   ├── aqueduct-server/        # TransportServer/TransportSession trait + セッション管理
|   └── aqueduct-server-ws/     # axum WebSocket 実装
└── frontend/                     # React + React Flow
    └── src/
        ├── components/           # NodeEditor, Palette, PropertyPanel, PinValueOverlay
        ├── hooks/                # useWebSocket, useGraphSync
        ├── protocol/             # Transport 抽象 + WebSocketTransport + TauriTransport
        └── stores/               # Zustand: グラフ状態 + ピン値
```

### このリポジトリ: `character-engine`

```
character-engine/
├── character-engine/
|   └── components/
|       └── aqueduct/           # ce-node-graph クレート
|           └── src/
|               ├── config.rs     # NodeGraphConfig + Provide<T>
|               ├── component.rs  # ManagedComponent 実装
|               ├── bridge/       # ブリッジノード群
|               |   ├── event_source.rs   # EventContext 受信ノード + EventContext 送信ノード
|               |   ├── discord.rs
|               |   ├── twitter.rs
|               |   ├── markov.rs
|               |   ├── llm.rs
|               |   ├── supabase.rs
|               |   └── earthquake.rs
|               └── registry.rs   # ブリッジノード一括登録
├── characters/                   # 既存キャラクターバイナリ（変更なし）
└── tools/
    └── node-editor/              # 統合バイナリ
        └── src/
            ├── main.rs
            ├── config.rs
            └── runtime.rs
```

### 依存グラフ

```
aqueduct-protocol  (依存なし)
  ^
aqueduct-core  (tokio, serde, chrono, uuid, arc-swap)
  ^
  ├── aqueduct-nodes
  ├── aqueduct-wasm-host  (wasmtime, wit-bindgen)
  ├── aqueduct-server     (TransportServer/TransportSession trait)
  |     ^
  |     └── aqueduct-server-ws  (axum)
  ^
ce-node-graph  (character-engine + aqueduct-core + aqueduct-server)
  ^
tools/node-editor  (ce-node-graph + character-engine コンポーネント群)
```

## character-engine 統合

### ManagedComponent 実装

```rust
pub struct NodeGraphComponent {
    live_graph: Arc<LiveGraph>,
    server: Arc<dyn TransportServer>,
    registry: Arc<NodeRegistry>,
}

impl ManagedComponent for NodeGraphComponent {
    fn name(&self) -> &str { "node-graph" }

    fn start(&self, ctx: ComponentContext) -> AppResult<Option<ComponentTask>> {
        let live_graph = Arc::clone(&self.live_graph);
        let server = Arc::clone(&self.server);
        let shutdown = ctx.shutdown.clone();

        Ok(Some(tokio::spawn(async move {
            let mut set = tokio::task::JoinSet::new();
            set.spawn(live_graph.run_tick_loop(shutdown.child_token()));
            set.spawn(server.run_accept_loop(Arc::clone(&live_graph), shutdown.child_token()));

            tokio::select! {
                _ = shutdown.cancelled() => Ok(()),
                Some(result) = set.join_next() => {
                    // いずれかのループが異常終了した場合、エラーを伝搬
                    result.map_err(|e| AppError::new(
                        ErrorKind::Infra,
                        ErrorCode::new("NODE_GRAPH_TASK_JOIN_ERROR"),
                        format!("node-graph task join error: {e}"),
                    ))?
                }
            }
        })))
    }
}
```

### イベントバスブリッジ

character-engine の `EventContext` との双方向接続。

```rust
/// EventContext → ノードグラフ（受信ノード）
/// character-engine のイベントバスを subscribe し、
/// 受信した EventContext を PinStore に書き込む
struct EventBridgeInNode {
    rx: broadcast::Receiver<EventContext>,
    latest: Option<EventContext>,
}

/// ノードグラフ → EventContext（送信ノード）
/// ノードグラフの出力を EventContext に変換して
/// character-engine のイベントバスに emit する
struct EventBridgeOutNode {
    tx: broadcast::Sender<EventContext>,
}
```

### ブリッジノード登録

```rust
pub fn register_bridge_nodes(
    registry: &mut NodeRegistry,
    container: &ComponentContainer,
) -> AppResult<()> {
    // 必須ブリッジ（なければエラー）
    // → 現状は全て optional（tools/node-editor の config で選択）

    // 利用可能なコンポーネントだけノードとして登録（ログ出力付き）
    if let Some(discord) = container.get::<DiscordClient>() {
        tracing::info!("Registering Discord bridge nodes");
        registry.register(Arc::new(DiscordSendNodeFactory::new(discord)));
    } else {
        tracing::info!("Discord not available, skipping bridge nodes");
    }

    if let Some(twitter) = container.get::<TwitterClient>() {
        tracing::info!("Registering Twitter bridge nodes");
        registry.register(Arc::new(TwitterPostNodeFactory::new(twitter)));
    } else {
        tracing::info!("Twitter not available, skipping bridge nodes");
    }

    // ... 他のコンポーネントも同様

    Ok(())
}
```

## テスト戦略

### 決定性テスト

- 同じグラフ + 同じ入力シーケンス → 同じ出力を保証
- tick カウンタをシードにして再現可能なテスト
- 非同期ジョブのモック（即座に Ready を返す）で決定性を検証

### ホットパッチ整合性テスト

- tick 中にパッチを投入 → 現 tick は旧グラフで完走、次 tick から新グラフ
- ノード削除後の非同期ジョブ完了 → 世代管理で結果が破棄されることを検証
- PinStore の値引き継ぎ → パッチ前後で未変更ピンの値が保持されることを検証

### 多クライアントテスト

- 複数 TransportSession が同時接続
- 各セッションの pin.subscribe が独立であることを検証
- graph.mutate の競合（同時パッチ）→ graph_rev による楽観的ロック検証
