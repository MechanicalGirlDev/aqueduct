import { useMemo } from 'react';
import { useGraphStore } from '../stores/graphStore';
import type { Property } from '../types';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Input } from '@/components/ui/input';
import { Textarea } from '@/components/ui/textarea';
import { Checkbox } from '@/components/ui/checkbox';
import { Label } from '@/components/ui/label';
import { Separator } from '@/components/ui/separator';

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

    const fieldLabel = (
      <div className="mb-1">
        <Label className="text-xs">{property.name}</Label>
        {property.description !== undefined && (
          <p className="text-[11px] text-muted-foreground mt-0.5">{property.description}</p>
        )}
      </div>
    );

    if (typeof value === 'boolean') {
      return (
        <div key={property.key} className="mb-3.5">
          {fieldLabel}
          <div className="flex items-center gap-2">
            <Checkbox
              id={`prop-${property.key}`}
              checked={value}
              onCheckedChange={(checked) => updateProperty(selectedNode.id, property.key, checked === true)}
            />
            <Label htmlFor={`prop-${property.key}`} className="text-xs text-muted-foreground">
              {value ? 'Enabled' : 'Disabled'}
            </Label>
          </div>
        </div>
      );
    }

    if (typeof value === 'number') {
      return (
        <div key={property.key} className="mb-3.5">
          {fieldLabel}
          <Input
            type="number"
            value={value}
            onChange={(event) => {
              const parsed = Number(event.target.value);
              updateProperty(selectedNode.id, property.key, Number.isNaN(parsed) ? 0 : parsed);
            }}
          />
        </div>
      );
    }

    if (typeof value === 'string' && (value.includes('\n') || value.length > 80)) {
      return (
        <div key={property.key} className="mb-3.5">
          {fieldLabel}
          <Textarea
            className="min-h-[90px] resize-y font-mono text-sm"
            value={value}
            onChange={(event) => updateProperty(selectedNode.id, property.key, event.target.value)}
          />
        </div>
      );
    }

    if (typeof value === 'string') {
      return (
        <div key={property.key} className="mb-3.5">
          {fieldLabel}
          <Input
            type="text"
            value={value}
            onChange={(event) => updateProperty(selectedNode.id, property.key, event.target.value)}
          />
        </div>
      );
    }

    return (
      <div key={property.key} className="mb-3.5">
        {fieldLabel}
        <Textarea
          className="min-h-[100px] resize-y font-mono text-sm"
          value={toText(value)}
          onChange={(event) =>
            updateProperty(selectedNode.id, property.key, parseUnknownFromText(event.target.value))
          }
        />
      </div>
    );
  };

  return (
    <ScrollArea className="h-full bg-sidebar">
      <div className="p-3.5">
        <div className="text-xs text-muted-foreground uppercase mb-2">Properties</div>

        {selectedNode === undefined ? (
          <p className="text-sm text-muted-foreground">Select a node to edit properties.</p>
        ) : (
          <>
            <div className="mb-2.5">
              <div className="text-sm font-bold text-foreground">{selectedNode.data.typeName}</div>
              <div className="text-[11px] text-muted-foreground">Node ID: {selectedNode.id}</div>
            </div>

            <Separator className="mb-3" />

            {(nodeDef?.properties ?? []).length === 0 ? (
              <p className="text-sm text-muted-foreground">This node has no editable properties.</p>
            ) : (
              nodeDef?.properties.map(renderField)
            )}
          </>
        )}
      </div>
    </ScrollArea>
  );
}
