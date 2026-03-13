import { useState } from 'react';
import type { Transport } from '../protocol/transport';
import { nextRequestId, useGraphStore } from '../stores/graphStore';
import { Button } from '@/components/ui/button';
import { Slider } from '@/components/ui/slider';
import { Badge } from '@/components/ui/badge';
import { ModeToggle } from '@/components/mode-toggle';

interface RuntimeControlsProps {
  transport: Transport;
  connected: boolean;
}

export function RuntimeControls({ transport, connected }: RuntimeControlsProps) {
  const runtimeState = useGraphStore((state) => state.runtimeState);
  const graphRev = useGraphStore((state) => state.graphRev);
  const [tickRate, setTickRate] = useState(30);

  const sendRuntimeStart = (): void => {
    transport.send({ request_id: nextRequestId(), body: { type: 'runtime.start' } });
  };

  const sendRuntimeStop = (): void => {
    transport.send({ request_id: nextRequestId(), body: { type: 'runtime.stop' } });
  };

  const sendTickRate = (hz: number): void => {
    transport.send({
      request_id: nextRequestId(),
      body: { type: 'runtime.set_tick_rate', hz },
    });
  };

  return (
    <div className="flex h-14 items-center gap-3 border-b border-border bg-card px-3.5">
      <Button size="sm" onClick={sendRuntimeStart}>
        Start
      </Button>
      <Button variant="destructive" size="sm" onClick={sendRuntimeStop}>
        Stop
      </Button>

      <div className="flex items-center gap-2 min-w-[220px]">
        <span className="text-xs text-muted-foreground">TickRate</span>
        <Slider
          min={1}
          max={120}
          step={1}
          value={[tickRate]}
          onValueChange={([hz]) => {
            if (hz !== undefined) {
              setTickRate(hz);
              sendTickRate(hz);
            }
          }}
          className="flex-1"
        />
        <span className="text-xs text-muted-foreground w-11 tabular-nums">{tickRate} Hz</span>
      </div>

      <div className="ml-auto flex items-center gap-2">
        <Badge variant={connected ? 'default' : 'secondary'}>
          {connected ? 'Connected' : 'Local mode'}
        </Badge>
        <Badge
          variant="outline"
          className={
            runtimeState === 'running'
              ? 'border-green-500 text-green-600 dark:text-green-400'
              : runtimeState === 'error'
                ? 'border-red-500 text-red-600 dark:text-red-400'
                : ''
          }
        >
          Runtime: {runtimeState}
        </Badge>
        <span className="text-xs text-muted-foreground tabular-nums">
          Rev {graphRev}
        </span>
        <ModeToggle />
      </div>
    </div>
  );
}
