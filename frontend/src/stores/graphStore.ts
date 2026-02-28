import {
  addEdge,
  applyEdgeChanges,
  applyNodeChanges,
  type Connection,
  type Edge as RFEdge,
  type Node as RFNode,
  type OnConnect,
  type OnEdgesChange,
  type OnNodesChange,
} from '@xyflow/react';
import { create } from 'zustand';
import { v4 as uuidv4 } from 'uuid';
import type {
  ClientMessage,
  Graph,
  NodeDef,
  NodeId,
  NodeInstance,
  PinId,
  RuntimeState,
} from '../types';
import type { Transport } from '../protocol/transport';

export type NodeCategory = 'math' | 'string' | 'logic' | 'time' | 'convert' | 'other';

export type CustomNodeData = {
  nodeId: NodeId;
  typeName: string;
  nodeDef?: NodeDef;
  properties: Record<string, unknown>;
  category: NodeCategory;
  [key: string]: unknown;
};

export type AqueductNode = RFNode<CustomNodeData, 'custom'>;

let transport: Transport | null = null;
let requestIdCounter = 1;
const propertyPatchTimers = new Map<string, ReturnType<typeof setTimeout>>();

const localFallbackNodeDefs: NodeDef[] = [
  {
    type_name: 'math.add',
    inputs: [
      { id: 'a', name: 'A', pin_type: 'float', direction: 'input' },
      { id: 'b', name: 'B', pin_type: 'float', direction: 'input' },
    ],
    outputs: [{ id: 'sum', name: 'Sum', pin_type: 'float', direction: 'output' }],
    properties: [{ key: 'scale', name: 'Scale', default_value: 1 }],
  },
  {
    type_name: 'string.concat',
    inputs: [
      { id: 'left', name: 'Left', pin_type: 'string', direction: 'input' },
      { id: 'right', name: 'Right', pin_type: 'string', direction: 'input' },
    ],
    outputs: [{ id: 'text', name: 'Text', pin_type: 'string', direction: 'output' }],
    properties: [{ key: 'separator', name: 'Separator', default_value: ' ' }],
  },
  {
    type_name: 'logic.and',
    inputs: [
      { id: 'lhs', name: 'LHS', pin_type: 'bool', direction: 'input' },
      { id: 'rhs', name: 'RHS', pin_type: 'bool', direction: 'input' },
    ],
    outputs: [{ id: 'value', name: 'Value', pin_type: 'bool', direction: 'output' }],
    properties: [],
  },
  {
    type_name: 'time.tick',
    inputs: [],
    outputs: [{ id: 'tick', name: 'Tick', pin_type: 'event', direction: 'output' }],
    properties: [{ key: 'interval_ms', name: 'Interval ms', default_value: 33 }],
  },
  {
    type_name: 'convert.to_string',
    inputs: [{ id: 'value', name: 'Value', pin_type: 'any', direction: 'input' }],
    outputs: [{ id: 'text', name: 'Text', pin_type: 'string', direction: 'output' }],
    properties: [{ key: 'pretty', name: 'Pretty JSON', default_value: true }],
  },
];

const categoryFromTypeName = (typeName: string): NodeCategory => {
  const head = typeName.split('.')[0] ?? 'other';
  if (head === 'math' || head === 'string' || head === 'logic' || head === 'time' || head === 'convert') {
    return head;
  }
  return 'other';
};

const makeNodeData = (
  instance: Pick<NodeInstance, 'id' | 'type_name' | 'properties'>,
  nodeDef: NodeDef | undefined
): CustomNodeData => ({
  nodeId: instance.id,
  typeName: instance.type_name,
  nodeDef,
  properties: { ...instance.properties },
  category: categoryFromTypeName(instance.type_name),
});

const sendMessage = (body: ClientMessage): void => {
  if (transport === null || !transport.connected) {
    return;
  }

  transport.send({ request_id: nextRequestId(), body });
};

export const nextRequestId = (): number => {
  const id = requestIdCounter;
  requestIdCounter += 1;
  return id;
};

export const setGraphTransport = (nextTransport: Transport | null): void => {
  transport = nextTransport;
};

export const toRuntimePinId = (nodeId: NodeId, pinId: PinId): PinId => `${nodeId}:${pinId}`;

interface GraphState {
  nodes: AqueductNode[];
  edges: RFEdge[];
  nodeDefs: NodeDef[];
  graphRev: number;
  runtimeState: RuntimeState;
  evalOrder: NodeId[];
  selectedNodeId: NodeId | null;
  onNodesChange: OnNodesChange<AqueductNode>;
  onEdgesChange: OnEdgesChange<RFEdge>;
  onConnect: OnConnect;
  setNodeDefs: (defs: NodeDef[]) => void;
  addNode: (typeName: string, position: { x: number; y: number }) => void;
  removeNode: (id: NodeId) => void;
  updateProperty: (nodeId: NodeId, key: string, value: unknown) => void;
  setSelectedNode: (id: NodeId | null) => void;
  loadGraphFromServer: (graph: Graph) => void;
  setRuntimeState: (state: RuntimeState) => void;
  setEvalOrder: (order: NodeId[]) => void;
  setGraphRev: (rev: number) => void;
}

export const useGraphStore = create<GraphState>((set, get) => ({
  nodes: [],
  edges: [],
  nodeDefs: localFallbackNodeDefs,
  graphRev: 0,
  runtimeState: 'stopped',
  evalOrder: [],
  selectedNodeId: null,
  onNodesChange: (changes) => {
    for (const change of changes) {
      if (change.type === 'remove') {
        sendMessage({
          type: 'graph.mutate',
          mutations: [{ type: 'remove_node', id: change.id }],
        });
      }
    }

    set((state) => ({
      nodes: applyNodeChanges(changes, state.nodes),
    }));
  },
  onEdgesChange: (changes) => {
    for (const change of changes) {
      if (change.type === 'remove') {
        sendMessage({
          type: 'graph.mutate',
          mutations: [{ type: 'remove_edge', id: change.id }],
        });
      }
    }

    set((state) => ({
      edges: applyEdgeChanges(changes, state.edges),
    }));
  },
  onConnect: (connection: Connection) => {
    const { source, sourceHandle, target, targetHandle } = connection;
    if (source === null || target === null || sourceHandle === null || targetHandle === null) {
      return;
    }

    const edgeId = uuidv4();
    const newEdge: RFEdge = {
      id: edgeId,
      source,
      sourceHandle,
      target,
      targetHandle,
    };

    set((state) => ({
      edges: addEdge(newEdge, state.edges),
    }));

    sendMessage({
      type: 'graph.mutate',
      mutations: [
        {
          type: 'add_edge',
          edge: {
            id: edgeId,
            from_node: source,
            from_pin: sourceHandle,
            to_node: target,
            to_pin: targetHandle,
          },
        },
      ],
    });
  },
  setNodeDefs: (defs) => {
    set((state) => ({
      nodeDefs: defs,
      nodes: state.nodes.map((node) => {
        const nodeDef = defs.find((def) => def.type_name === node.data.typeName);
        return {
          ...node,
          data: {
            ...node.data,
            nodeDef,
            category: categoryFromTypeName(node.data.typeName),
          },
        };
      }),
    }));
  },
  addNode: (typeName, position) => {
    const instanceId = uuidv4();
    const nodeDef = get().nodeDefs.find((def) => def.type_name === typeName);
    const properties = Object.fromEntries(
      (nodeDef?.properties ?? []).map((property) => [property.key, property.default_value])
    );

    const instance: NodeInstance = {
      id: instanceId,
      type_name: typeName,
      properties,
      position: [position.x, position.y],
    };

    const node: AqueductNode = {
      id: instanceId,
      type: 'custom',
      position,
      data: makeNodeData(instance, nodeDef),
    };

    set((state) => ({ nodes: [...state.nodes, node] }));

    sendMessage({
      type: 'graph.mutate',
      mutations: [{ type: 'add_node', instance }],
    });
  },
  removeNode: (id) => {
    set((state) => ({
      nodes: state.nodes.filter((node) => node.id !== id),
      edges: state.edges.filter((edge) => edge.source !== id && edge.target !== id),
      selectedNodeId: state.selectedNodeId === id ? null : state.selectedNodeId,
    }));

    sendMessage({
      type: 'graph.mutate',
      mutations: [{ type: 'remove_node', id }],
    });
  },
  updateProperty: (nodeId, key, value) => {
    set((state) => ({
      nodes: state.nodes.map((node) => {
        if (node.id !== nodeId) {
          return node;
        }

        return {
          ...node,
          data: {
            ...node.data,
            properties: {
              ...node.data.properties,
              [key]: value,
            },
          },
        };
      }),
    }));

    const debounceKey = `${nodeId}:${key}`;
    const current = propertyPatchTimers.get(debounceKey);
    if (current !== undefined) {
      clearTimeout(current);
    }

    propertyPatchTimers.set(
      debounceKey,
      setTimeout(() => {
        sendMessage({
          type: 'graph.mutate',
          mutations: [{ type: 'update_property', node_id: nodeId, key, value }],
        });
        propertyPatchTimers.delete(debounceKey);
      }, 100)
    );
  },
  setSelectedNode: (id) => {
    set({ selectedNodeId: id });
  },
  loadGraphFromServer: (graph) => {
    const defs = get().nodeDefs;
    const nodes = Object.values(graph.nodes).map((instance) => {
      const nodeDef = defs.find((def) => def.type_name === instance.type_name);
      return {
        id: instance.id,
        type: 'custom',
        position: { x: instance.position[0], y: instance.position[1] },
        data: makeNodeData(instance, nodeDef),
      } satisfies AqueductNode;
    });

    const edges: RFEdge[] = graph.edges.map((edge) => ({
      id: edge.id,
      source: edge.from_node,
      sourceHandle: edge.from_pin,
      target: edge.to_node,
      targetHandle: edge.to_pin,
    }));

    set({ nodes, edges });
  },
  setRuntimeState: (state) => set({ runtimeState: state }),
  setEvalOrder: (order) => set({ evalOrder: order }),
  setGraphRev: (rev) => set({ graphRev: rev }),
}));
