import { useCallback, type DragEvent } from 'react';
import {
  Background,
  Controls,
  MiniMap,
  ReactFlow,
  ReactFlowProvider,
  useReactFlow,
  type NodeTypes,
} from '@xyflow/react';
import { CustomNode } from './CustomNode';
import { useGraphStore, type AqueductNode } from '../stores/graphStore';

const PALETTE_MIME = 'application/aqueduct-node-type';

const nodeTypes: NodeTypes = {
  custom: CustomNode,
};

const miniMapColor = (node: AqueductNode): string => {
  const category = node.data.category;
  switch (category) {
    case 'math':
      return '#2b6cb0';
    case 'string':
      return '#2f855a';
    case 'logic':
      return '#b7791f';
    case 'time':
      return '#6b46c1';
    case 'convert':
      return '#c05621';
    default:
      return '#4a5568';
  }
};

function NodeEditorCanvas() {
  const nodes = useGraphStore((state) => state.nodes);
  const edges = useGraphStore((state) => state.edges);
  const onNodesChange = useGraphStore((state) => state.onNodesChange);
  const onEdgesChange = useGraphStore((state) => state.onEdgesChange);
  const onConnect = useGraphStore((state) => state.onConnect);
  const addNode = useGraphStore((state) => state.addNode);
  const setSelectedNode = useGraphStore((state) => state.setSelectedNode);

  const { screenToFlowPosition } = useReactFlow();

  const onDragOver = useCallback((event: DragEvent<HTMLDivElement>) => {
    event.preventDefault();
    event.dataTransfer.dropEffect = 'copy';
  }, []);

  const onDrop = useCallback(
    (event: DragEvent<HTMLDivElement>) => {
      event.preventDefault();

      const typeName = event.dataTransfer.getData(PALETTE_MIME);
      if (typeName === '') {
        return;
      }

      const position = screenToFlowPosition({ x: event.clientX, y: event.clientY });
      addNode(typeName, position);
    },
    [addNode, screenToFlowPosition]
  );

  return (
    <ReactFlow
      nodes={nodes}
      edges={edges}
      nodeTypes={nodeTypes}
      onNodesChange={onNodesChange}
      onEdgesChange={onEdgesChange}
      onConnect={onConnect}
      onDragOver={onDragOver}
      onDrop={onDrop}
      onSelectionChange={({ nodes: selectedNodes }) => {
        setSelectedNode(selectedNodes[0]?.id ?? null);
      }}
      fitView
      proOptions={{ hideAttribution: true }}
      style={{ background: 'linear-gradient(180deg, #0f172a 0%, #111827 100%)' }}
    >
      <MiniMap
        nodeColor={miniMapColor}
        pannable
        zoomable
        style={{ backgroundColor: '#0b1020', border: '1px solid #1f2937' }}
      />
      <Controls style={{ background: '#0b1020', border: '1px solid #1f2937' }} />
      <Background gap={20} size={1} color="#1f2937" />
    </ReactFlow>
  );
}

export function NodeEditor() {
  return (
    <div style={{ height: '100%', width: '100%' }}>
      <ReactFlowProvider>
        <NodeEditorCanvas />
      </ReactFlowProvider>
    </div>
  );
}
