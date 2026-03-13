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
import { getCategoryColor } from '@/lib/category-colors';

const PALETTE_MIME = 'application/aqueduct-node-type';

const nodeTypes: NodeTypes = {
  custom: CustomNode,
};

const miniMapColor = (node: AqueductNode): string => getCategoryColor(node.data.category);

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
      className="bg-background"
    >
      <MiniMap nodeColor={miniMapColor} pannable zoomable />
      <Controls />
      <Background gap={20} size={1} />
    </ReactFlow>
  );
}

export function NodeEditor() {
  return (
    <div className="h-full w-full">
      <ReactFlowProvider>
        <NodeEditorCanvas />
      </ReactFlowProvider>
    </div>
  );
}
