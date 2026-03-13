import { useEffect, useState } from 'react';
import type { Transport } from '../protocol/transport';
import { TauriTransport } from '../protocol/tauri';
import { WebSocketTransport } from '../protocol/websocket';
import { useGraphStore, nextRequestId, setGraphTransport } from '../stores/graphStore';
import { usePinStore } from '../stores/pinStore';
import type { ServerEnvelope } from '../types';

function isTauri(): boolean {
  return '__TAURI_INTERNALS__' in window;
}

const sharedTransport: Transport = isTauri()
  ? new TauriTransport()
  : new WebSocketTransport('ws://localhost:9400/ws');
let initialized = false;
let connectInFlight: Promise<void> | null = null;

const routeServerMessage = (msg: ServerEnvelope): void => {
  const graphState = useGraphStore.getState();
  const pinState = usePinStore.getState();

  graphState.setGraphRev(msg.graph_rev);

  switch (msg.body.type) {
    case 'registry.nodes':
      graphState.setNodeDefs(msg.body.defs);
      break;
    case 'pin.values':
      pinState.setPinValues(msg.body.values);
      break;
    case 'runtime.state':
      graphState.setRuntimeState(msg.body.state);
      break;
    case 'graph.compiled':
      graphState.setEvalOrder(msg.body.eval_order);
      if (msg.body.warnings.length > 0) {
        console.warn('graph compilation warnings', msg.body.warnings);
      }
      break;
    case 'error':
      console.error(`server error [${msg.body.code}]`, msg.body.message, msg.body.node_id);
      break;
    case 'handshake':
      break;
    default: {
      const unknownMessage: never = msg.body;
      console.warn('unhandled server message', unknownMessage);
    }
  }
};

const ensureConnected = async (): Promise<void> => {
  if (sharedTransport.connected) {
    return;
  }

  if (connectInFlight !== null) {
    await connectInFlight;
    return;
  }

  connectInFlight = sharedTransport
    .connect()
    .then(() => {
      sharedTransport.send({
        request_id: nextRequestId(),
        body: { type: 'registry.list' },
      });
    })
    .finally(() => {
      connectInFlight = null;
    });

  await connectInFlight;
};

export const getSharedTransport = (): Transport => sharedTransport;

export function useTransport(): { transport: Transport; connected: boolean } {
  const [connected, setConnected] = useState(sharedTransport.connected);

  useEffect(() => {
    setGraphTransport(sharedTransport);

    if (!initialized) {
      initialized = true;
      sharedTransport.onMessage(routeServerMessage);

      ensureConnected().catch((error) => {
        console.warn('server unavailable, running in local-only mode', error);
      });
    }

    const interval = window.setInterval(() => {
      if (!sharedTransport.connected && connectInFlight === null) {
        ensureConnected().catch(() => {
          // Keep local-only mode while backend is unavailable.
        });
      }
      setConnected(sharedTransport.connected);
    }, 1000);

    return () => {
      window.clearInterval(interval);
    };
  }, []);

  return { transport: sharedTransport, connected };
}
