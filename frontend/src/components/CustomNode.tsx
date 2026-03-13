import { Handle, Position, type NodeProps } from '@xyflow/react';
import { PinValueOverlay } from './PinValueOverlay';
import { toRuntimePinId, type AqueductNode } from '../stores/graphStore';
import { cn } from '@/lib/utils';
import { categoryBg } from '@/lib/category-colors';

export function CustomNode({ data, selected }: NodeProps<AqueductNode>) {
  const inputs = data.nodeDef?.inputs ?? [];
  const outputs = data.nodeDef?.outputs ?? [];

  return (
    <div
      className={cn(
        'min-w-[230px] rounded-lg overflow-hidden bg-card text-card-foreground',
        'shadow-md border',
        selected
          ? 'border-primary ring-2 ring-primary/25'
          : 'border-border',
      )}
    >
      <div
        className={cn(
          'px-2.5 py-2 text-xs font-bold uppercase tracking-wide text-white',
          categoryBg[data.category],
        )}
      >
        {data.typeName}
      </div>

      <div className="grid grid-cols-2 gap-3 p-2.5">
        <div>
          {inputs.length === 0 ? (
            <div className="text-[11px] text-muted-foreground">No inputs</div>
          ) : (
            inputs.map((pin) => (
              <div
                key={`in-${pin.id}`}
                className="relative text-[11px] py-1 pl-3.5 text-muted-foreground"
              >
                <Handle
                  type="target"
                  id={pin.id}
                  position={Position.Left}
                  className="!w-2 !h-2 !bg-muted-foreground !border !border-border"
                />
                {pin.name}
              </div>
            ))
          )}
        </div>

        <div>
          {outputs.length === 0 ? (
            <div className="text-[11px] text-muted-foreground text-right">No outputs</div>
          ) : (
            outputs.map((pin) => (
              <div
                key={`out-${pin.id}`}
                className="relative text-[11px] py-1 pr-3.5 text-muted-foreground flex justify-end items-center"
              >
                <span>{pin.name}</span>
                <PinValueOverlay pinId={toRuntimePinId(data.nodeId, pin.id)} />
                <Handle
                  type="source"
                  id={pin.id}
                  position={Position.Right}
                  className="!w-2 !h-2 !bg-muted-foreground !border !border-border"
                />
              </div>
            ))
          )}
        </div>
      </div>
    </div>
  );
}
