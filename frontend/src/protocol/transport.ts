import type { ClientEnvelope, ServerEnvelope } from '../types';

export interface Transport {
  send(msg: ClientEnvelope): void;
  onMessage(handler: (msg: ServerEnvelope) => void): void;
  connect(): Promise<void>;
  disconnect(): void;
  readonly connected: boolean;
}
