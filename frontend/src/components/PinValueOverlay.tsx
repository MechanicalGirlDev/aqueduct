import type { PinId, PinValue } from '../types';
import { usePinStore } from '../stores/pinStore';
import { Badge } from '@/components/ui/badge';

interface PinValueOverlayProps {
  pinId: PinId;
}

const formatPinValue = (value: PinValue): string => {
  switch (value.kind) {
    case 'float':
      return Number.isFinite(value.value) ? value.value.toFixed(3) : 'NaN';
    case 'int':
      return String(Math.trunc(value.value));
    case 'bool':
      return value.value ? 'true' : 'false';
    case 'string': {
      const text = value.value.trim();
      return text.length > 18 ? `${text.slice(0, 18)}...` : text;
    }
    case 'json': {
      const text = JSON.stringify(value.value);
      if (text === undefined) {
        return 'undefined';
      }
      return text.length > 18 ? `${text.slice(0, 18)}...` : text;
    }
    case 'event':
      return 'event';
    case 'none':
      return 'none';
    default: {
      const neverValue: never = value;
      return String(neverValue);
    }
  }
};

export function PinValueOverlay({ pinId }: PinValueOverlayProps) {
  const value = usePinStore((state) => state.pinValues[pinId]);

  if (value === undefined) {
    return null;
  }

  return (
    <Badge variant="outline" className="ml-2 text-[10px] leading-relaxed font-mono whitespace-nowrap" title={formatPinValue(value)}>
      {formatPinValue(value)}
    </Badge>
  );
}
