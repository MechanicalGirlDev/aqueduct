import { useCallback, useEffect, useRef } from 'react';
import { getSharedTransport } from './useTransport';
import { nextRequestId, toRuntimePinId, useGraphStore } from '../stores/graphStore';
import type { PinId } from '../types';

export function useGraphSync(): void {
  const nodes = useGraphStore((state) => state.nodes);
  const subscribedPinIdsRef = useRef<Set<PinId>>(new Set());
  const previousConnectedRef = useRef<boolean>(false);

  const syncPinSubscriptions = useCallback(() => {
    const transport = getSharedTransport();
    const desiredPinIds = new Set<PinId>();

    for (const node of nodes) {
      for (const pin of node.data.nodeDef?.outputs ?? []) {
        desiredPinIds.add(toRuntimePinId(node.id, pin.id));
      }
    }

    if (!transport.connected) {
      previousConnectedRef.current = false;
      return;
    }

    if (!previousConnectedRef.current) {
      // On fresh reconnect, backend session state is empty.
      subscribedPinIdsRef.current = new Set();
    }

    const subscribedPinIds = subscribedPinIdsRef.current;
    const subscribeIds = [...desiredPinIds].filter((id) => !subscribedPinIds.has(id));
    const unsubscribeIds = [...subscribedPinIds].filter((id) => !desiredPinIds.has(id));

    if (subscribeIds.length > 0) {
      transport.send({
        request_id: nextRequestId(),
        body: { type: 'pin.subscribe', pin_ids: subscribeIds },
      });
    }

    if (unsubscribeIds.length > 0) {
      transport.send({
        request_id: nextRequestId(),
        body: { type: 'pin.unsubscribe', pin_ids: unsubscribeIds },
      });
    }

    subscribedPinIdsRef.current = desiredPinIds;
    previousConnectedRef.current = true;
  }, [nodes]);

  useEffect(() => {
    syncPinSubscriptions();
  }, [syncPinSubscriptions]);

  useEffect(() => {
    const interval = window.setInterval(syncPinSubscriptions, 1000);
    return () => {
      window.clearInterval(interval);
    };
  }, [syncPinSubscriptions]);

  useEffect(
    () => () => {
      const transport = getSharedTransport();
      const remaining = [...subscribedPinIdsRef.current];
      if (transport.connected && remaining.length > 0) {
        transport.send({
          request_id: nextRequestId(),
          body: { type: 'pin.unsubscribe', pin_ids: remaining },
        });
      }
    },
    []
  );
}
