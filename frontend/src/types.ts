// ID types
export type PinId = string;
export type NodeId = string;
export type EdgeId = string;

// Pin types
export type PinType = 'float' | 'int' | 'bool' | 'string' | 'json' | 'event' | 'any';

// Pin value tagged union
export type PinValue =
  | { kind: 'float'; value: number }
  | { kind: 'int'; value: number }
  | { kind: 'bool'; value: boolean }
  | { kind: 'string'; value: string }
  | { kind: 'json'; value: unknown }
  | { kind: 'event' }
  | { kind: 'none' };

export type Direction = 'input' | 'output';

export interface Property {
  key: string;
  name: string;
  description?: string;
  default_value: unknown;
}

export interface PinDef {
  id: PinId;
  name: string;
  pin_type: PinType;
  direction: Direction;
}

export interface NodeDef {
  type_name: string;
  inputs: PinDef[];
  outputs: PinDef[];
  properties: Property[];
}

export interface NodeInstance {
  id: NodeId;
  type_name: string;
  properties: Record<string, unknown>;
  position: [number, number];
}

export interface Edge {
  id: EdgeId;
  from_node: NodeId;
  from_pin: PinId;
  to_node: NodeId;
  to_pin: PinId;
}

export interface Graph {
  nodes: Record<NodeId, NodeInstance>;
  edges: Edge[];
}

export type GraphMutation =
  | { type: 'add_node'; instance: NodeInstance }
  | { type: 'remove_node'; id: NodeId }
  | { type: 'add_edge'; edge: Edge }
  | { type: 'remove_edge'; id: EdgeId }
  | { type: 'update_property'; node_id: NodeId; key: string; value: unknown };

export type RuntimeState = 'running' | 'stopped' | 'error';

export type ClientMessage =
  | { type: 'handshake'; protocol_version: string }
  | { type: 'graph.mutate'; mutations: GraphMutation[] }
  | { type: 'graph.load'; graph: Graph }
  | { type: 'graph.save' }
  | { type: 'graph.compile' }
  | { type: 'runtime.start' }
  | { type: 'runtime.stop' }
  | { type: 'runtime.set_tick_rate'; hz: number }
  | { type: 'registry.list' }
  | { type: 'pin.subscribe'; pin_ids: PinId[] }
  | { type: 'pin.unsubscribe'; pin_ids: PinId[] };

export type ServerMessage =
  | { type: 'handshake'; protocol_version: string }
  | { type: 'pin.values'; values: Record<PinId, PinValue> }
  | { type: 'runtime.state'; state: RuntimeState }
  | { type: 'graph.compiled'; eval_order: NodeId[]; warnings: string[] }
  | { type: 'error'; code: string; message: string; node_id?: NodeId }
  | { type: 'registry.nodes'; defs: NodeDef[] };

export interface ClientEnvelope {
  request_id: number;
  body: ClientMessage;
}

export interface ServerEnvelope {
  request_id: number | null;
  body: ServerMessage;
  graph_rev: number;
}
