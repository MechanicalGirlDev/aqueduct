import { Handle, Position, type NodeProps } from '@xyflow/react';
import { PinValueOverlay } from './PinValueOverlay';
import { toRuntimePinId, type AqueductNode, type NodeCategory } from '../stores/graphStore';

const categoryColors: Record<NodeCategory, string> = {
  math: '#2b6cb0',
  string: '#2f855a',
  logic: '#b7791f',
  time: '#6b46c1',
  convert: '#c05621',
  other: '#4a5568',
};

export function CustomNode({ data, selected }: NodeProps<AqueductNode>) {
  const inputs = data.nodeDef?.inputs ?? [];
  const outputs = data.nodeDef?.outputs ?? [];

  return (
    <div
      style={{
        minWidth: 230,
        borderRadius: 10,
        border: selected ? '1px solid #fbbf24' : '1px solid #2d3748',
        boxShadow: selected ? '0 0 0 2px rgba(251, 191, 36, 0.25)' : '0 8px 18px rgba(0, 0, 0, 0.25)',
        overflow: 'hidden',
        background: '#0f172a',
        color: '#e2e8f0',
      }}
    >
      <div
        style={{
          background: categoryColors[data.category],
          fontWeight: 700,
          fontSize: 12,
          letterSpacing: 0.4,
          textTransform: 'uppercase',
          padding: '8px 10px',
        }}
      >
        {data.typeName}
      </div>

      <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 12, padding: 10 }}>
        <div>
          {inputs.length === 0 ? (
            <div style={{ fontSize: 11, color: '#718096' }}>No inputs</div>
          ) : (
            inputs.map((pin) => (
              <div
                key={`in-${pin.id}`}
                style={{
                  position: 'relative',
                  fontSize: 11,
                  padding: '4px 0 4px 14px',
                  color: '#a0aec0',
                }}
              >
                <Handle
                  type="target"
                  id={pin.id}
                  position={Position.Left}
                  style={{
                    width: 8,
                    height: 8,
                    background: '#cbd5e0',
                    border: '1px solid #4a5568',
                    top: '50%',
                    transform: 'translateY(-50%)',
                  }}
                />
                {pin.name}
              </div>
            ))
          )}
        </div>

        <div>
          {outputs.length === 0 ? (
            <div style={{ fontSize: 11, color: '#718096', textAlign: 'right' }}>No outputs</div>
          ) : (
            outputs.map((pin) => (
              <div
                key={`out-${pin.id}`}
                style={{
                  position: 'relative',
                  fontSize: 11,
                  padding: '4px 14px 4px 0',
                  color: '#a0aec0',
                  display: 'flex',
                  justifyContent: 'flex-end',
                  alignItems: 'center',
                }}
              >
                <span>{pin.name}</span>
                <PinValueOverlay pinId={toRuntimePinId(data.nodeId, pin.id)} />
                <Handle
                  type="source"
                  id={pin.id}
                  position={Position.Right}
                  style={{
                    width: 8,
                    height: 8,
                    background: '#cbd5e0',
                    border: '1px solid #4a5568',
                    top: '50%',
                    transform: 'translateY(-50%)',
                  }}
                />
              </div>
            ))
          )}
        </div>
      </div>
    </div>
  );
}
