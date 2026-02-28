import { useState } from 'react';
import type { Transport } from '../protocol/transport';
import { nextRequestId, useGraphStore } from '../stores/graphStore';

interface RuntimeControlsProps {
  transport: Transport;
  connected: boolean;
}

const stateColor: Record<'running' | 'stopped' | 'error', string> = {
  running: '#22c55e',
  stopped: '#94a3b8',
  error: '#ef4444',
};

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
    <div
      style={{
        height: 56,
        padding: '10px 14px',
        borderBottom: '1px solid #1f2937',
        background: '#0b1020',
        display: 'flex',
        alignItems: 'center',
        gap: 14,
      }}
    >
      <button
        type="button"
        onClick={sendRuntimeStart}
        style={{
          background: '#14532d',
          border: '1px solid #16a34a',
          color: '#dcfce7',
          borderRadius: 6,
          padding: '6px 10px',
          cursor: 'pointer',
        }}
      >
        Start
      </button>
      <button
        type="button"
        onClick={sendRuntimeStop}
        style={{
          background: '#3f1d1d',
          border: '1px solid #ef4444',
          color: '#fee2e2',
          borderRadius: 6,
          padding: '6px 10px',
          cursor: 'pointer',
        }}
      >
        Stop
      </button>

      <label style={{ display: 'flex', alignItems: 'center', gap: 8, minWidth: 220, color: '#cbd5e1' }}>
        <span style={{ fontSize: 12 }}>TickRate</span>
        <input
          type="range"
          min={1}
          max={120}
          value={tickRate}
          onChange={(event) => {
            const hz = Number(event.target.value);
            setTickRate(hz);
            sendTickRate(hz);
          }}
          style={{ flex: 1 }}
        />
        <span style={{ fontSize: 12, width: 44 }}>{tickRate} Hz</span>
      </label>

      <div style={{ marginLeft: 'auto', display: 'flex', alignItems: 'center', gap: 12, fontSize: 12 }}>
        <span style={{ color: connected ? '#22c55e' : '#f59e0b' }}>
          {connected ? 'Connected' : 'Local mode'}
        </span>
        <span style={{ color: stateColor[runtimeState] }}>Runtime: {runtimeState}</span>
        <span style={{ color: '#94a3b8' }}>GraphRev: {graphRev}</span>
      </div>
    </div>
  );
}
