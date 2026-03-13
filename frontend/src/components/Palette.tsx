import { useMemo, type DragEvent } from 'react';
import { useGraphStore, type NodeCategory } from '../stores/graphStore';
import type { NodeDef } from '../types';
import { ScrollArea } from '@/components/ui/scroll-area';
import { cn } from '@/lib/utils';
import { categoryText, categoryBorderLeft } from '@/lib/category-colors';

const PALETTE_MIME = 'application/aqueduct-node-type';

const categoryLabel: Record<NodeCategory, string> = {
  math: 'Math',
  string: 'String',
  logic: 'Logic',
  time: 'Time',
  convert: 'Convert',
  other: 'Other',
};

const getCategory = (typeName: string): NodeCategory => {
  const head = typeName.split('.')[0] ?? 'other';
  if (head === 'math' || head === 'string' || head === 'logic' || head === 'time' || head === 'convert') {
    return head;
  }
  return 'other';
};

const getInitialGroups = (): Record<NodeCategory, NodeDef[]> => ({
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
    <ScrollArea className="h-full bg-sidebar">
      <div className="p-3">
        <div className="mb-2.5 text-xs text-muted-foreground">Node Types</div>
        {(['math', 'string', 'logic', 'time', 'convert', 'other'] as const).map((category) => {
          const defs = grouped[category];
          if (defs.length === 0) {
            return null;
          }

          return (
            <section key={category} className="mb-3.5">
              <div className={cn('text-xs font-bold uppercase tracking-wide mb-1.5', categoryText[category])}>
                {categoryLabel[category]}
              </div>
              <div className="grid gap-1.5">
                {defs.map((def) => (
                  <div
                    key={def.type_name}
                    draggable
                    onDragStart={(event) => handleDragStart(event, def.type_name)}
                    className={cn(
                      'rounded-md border border-border border-l-4 px-2.5 py-2 text-xs',
                      'bg-card text-foreground cursor-grab select-none',
                      'hover:bg-accent transition-colors',
                      categoryBorderLeft[category],
                    )}
                    title={def.type_name}
                  >
                    {def.type_name}
                  </div>
                ))}
              </div>
            </section>
          );
        })}
      </div>
    </ScrollArea>
  );
}
