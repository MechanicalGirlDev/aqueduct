import { useMemo, type DragEvent } from 'react';
import { useGraphStore } from '../stores/graphStore';
import type { NodeDef } from '../types';

const PALETTE_MIME = 'application/aqueduct-node-type';

type PaletteCategory = 'math' | 'string' | 'logic' | 'time' | 'convert' | 'other';

const categoryLabel: Record<PaletteCategory, string> = {
  math: 'Math',
  string: 'String',
  logic: 'Logic',
  time: 'Time',
  convert: 'Convert',
  other: 'Other',
};

const categoryColor: Record<PaletteCategory, string> = {
  math: '#2b6cb0',
  string: '#2f855a',
  logic: '#b7791f',
  time: '#6b46c1',
  convert: '#c05621',
  other: '#4a5568',
};

const getCategory = (typeName: string): PaletteCategory => {
  const head = typeName.split('.')[0] ?? 'other';
  if (head === 'math' || head === 'string' || head === 'logic' || head === 'time' || head === 'convert') {
    return head;
  }
  return 'other';
};

const getInitialGroups = (): Record<PaletteCategory, NodeDef[]> => ({
  math: [],
  string: [],
  logic: [],
  time: [],
  convert: [],
  other: [],
});

export function Palette() {
  const nodeDefs = useGraphStore((state) => state.nodeDefs);

  const grouped = useMemo(() => {
    const groups = getInitialGroups();
    for (const def of nodeDefs) {
      groups[getCategory(def.type_name)].push(def);
    }
    return groups;
  }, [nodeDefs]);

  const handleDragStart = (event: DragEvent<HTMLDivElement>, typeName: string): void => {
    event.dataTransfer.setData(PALETTE_MIME, typeName);
    event.dataTransfer.effectAllowed = 'copy';
  };

  return (
    <aside style={{ padding: 12, overflowY: 'auto', height: '100%', background: '#121826' }}>
      <div style={{ marginBottom: 10, fontSize: 12, color: '#9ca3af' }}>Node Types</div>
      {(['math', 'string', 'logic', 'time', 'convert', 'other'] as const).map((category) => {
        const defs = grouped[category];
        if (defs.length === 0) {
          return null;
        }

        return (
          <section key={category} style={{ marginBottom: 14 }}>
            <div
              style={{
                color: categoryColor[category],
                fontWeight: 700,
                fontSize: 12,
                marginBottom: 6,
                textTransform: 'uppercase',
                letterSpacing: 0.5,
              }}
            >
              {categoryLabel[category]}
            </div>
            <div style={{ display: 'grid', gap: 6 }}>
              {defs.map((def) => (
                <div
                  key={def.type_name}
                  draggable
                  onDragStart={(event) => handleDragStart(event, def.type_name)}
                  style={{
                    border: '1px solid #2d3748',
                    borderLeft: `4px solid ${categoryColor[category]}`,
                    borderRadius: 6,
                    padding: '8px 10px',
                    fontSize: 12,
                    background: '#0f172a',
                    color: '#e5e7eb',
                    cursor: 'grab',
                    userSelect: 'none',
                  }}
                  title={def.type_name}
                >
                  {def.type_name}
                </div>
              ))}
            </div>
          </section>
        );
      })}
    </aside>
  );
}
