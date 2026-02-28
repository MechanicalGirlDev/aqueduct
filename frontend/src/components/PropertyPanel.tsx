import { useMemo, type CSSProperties } from 'react';
import { useGraphStore } from '../stores/graphStore';
import type { Property } from '../types';

const toText = (value: unknown): string => {
  if (typeof value === 'string') {
    return value;
  }
  if (typeof value === 'number' || typeof value === 'boolean') {
    return String(value);
  }

  const text = JSON.stringify(value, null, 2);
  return text ?? '';
};

const parseUnknownFromText = (text: string): unknown => {
  try {
    return JSON.parse(text);
  } catch {
    return text;
  }
};

const inputStyle: CSSProperties = {
  width: '100%',
  background: '#0f172a',
  border: '1px solid #334155',
  borderRadius: 6,
  padding: '8px 10px',
  color: '#e2e8f0',
  fontSize: 13,
  boxSizing: 'border-box',
};

export function PropertyPanel() {
  const selectedNodeId = useGraphStore((state) => state.selectedNodeId);
  const nodes = useGraphStore((state) => state.nodes);
  const nodeDefs = useGraphStore((state) => state.nodeDefs);
  const updateProperty = useGraphStore((state) => state.updateProperty);

  const selectedNode = useMemo(
    () => nodes.find((node) => node.id === selectedNodeId),
    [nodes, selectedNodeId]
  );

  const nodeDef = useMemo(() => {
    if (selectedNode === undefined) {
      return undefined;
    }

    return (
      selectedNode.data.nodeDef ?? nodeDefs.find((def) => def.type_name === selectedNode.data.typeName)
    );
  }, [nodeDefs, selectedNode]);

  const renderField = (property: Property) => {
    if (selectedNode === undefined) {
      return null;
    }

    const value = selectedNode.data.properties[property.key] ?? property.default_value;
    const commonLabel = (
      <>
        <div style={{ fontSize: 12, color: '#cbd5e1', marginBottom: 4 }}>{property.name}</div>
        {property.description !== undefined && (
          <div style={{ fontSize: 11, color: '#64748b', marginBottom: 6 }}>{property.description}</div>
        )}
      </>
    );

    if (typeof value === 'boolean') {
      return (
        <label key={property.key} style={{ display: 'block', marginBottom: 14 }}>
          {commonLabel}
          <input
            type="checkbox"
            checked={value}
            onChange={(event) => updateProperty(selectedNode.id, property.key, event.target.checked)}
          />
        </label>
      );
    }

    if (typeof value === 'number') {
      return (
        <label key={property.key} style={{ display: 'block', marginBottom: 14 }}>
          {commonLabel}
          <input
            style={inputStyle}
            type="number"
            value={value}
            onChange={(event) => {
              const parsed = Number(event.target.value);
              updateProperty(selectedNode.id, property.key, Number.isNaN(parsed) ? 0 : parsed);
            }}
          />
        </label>
      );
    }

    if (typeof value === 'string' && (value.includes('\n') || value.length > 80)) {
      return (
        <label key={property.key} style={{ display: 'block', marginBottom: 14 }}>
          {commonLabel}
          <textarea
            style={{ ...inputStyle, minHeight: 90, resize: 'vertical', fontFamily: 'ui-monospace, SFMono-Regular, Menlo, monospace' }}
            value={value}
            onChange={(event) => updateProperty(selectedNode.id, property.key, event.target.value)}
          />
        </label>
      );
    }

    if (typeof value === 'string') {
      return (
        <label key={property.key} style={{ display: 'block', marginBottom: 14 }}>
          {commonLabel}
          <input
            style={inputStyle}
            type="text"
            value={value}
            onChange={(event) => updateProperty(selectedNode.id, property.key, event.target.value)}
          />
        </label>
      );
    }

    return (
      <label key={property.key} style={{ display: 'block', marginBottom: 14 }}>
        {commonLabel}
        <textarea
          style={{ ...inputStyle, minHeight: 100, resize: 'vertical', fontFamily: 'ui-monospace, SFMono-Regular, Menlo, monospace' }}
          value={toText(value)}
          onChange={(event) =>
            updateProperty(selectedNode.id, property.key, parseUnknownFromText(event.target.value))
          }
        />
      </label>
    );
  };

  return (
    <aside style={{ padding: 14, height: '100%', overflowY: 'auto', background: '#101625' }}>
      <div style={{ fontSize: 12, color: '#94a3b8', marginBottom: 8, textTransform: 'uppercase' }}>
        Properties
      </div>

      {selectedNode === undefined ? (
        <div style={{ fontSize: 13, color: '#64748b' }}>Select a node to edit properties.</div>
      ) : (
        <>
          <div style={{ marginBottom: 10 }}>
            <div style={{ color: '#e2e8f0', fontWeight: 700, fontSize: 14 }}>{selectedNode.data.typeName}</div>
            <div style={{ color: '#64748b', fontSize: 11 }}>Node ID: {selectedNode.id}</div>
          </div>

          {(nodeDef?.properties ?? []).length === 0 ? (
            <div style={{ fontSize: 13, color: '#64748b' }}>This node has no editable properties.</div>
          ) : (
            nodeDef?.properties.map(renderField)
          )}
        </>
      )}
    </aside>
  );
}
