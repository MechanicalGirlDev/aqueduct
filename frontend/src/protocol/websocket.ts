import type { ClientEnvelope, ServerEnvelope } from '../types';
import type { Transport } from './transport';

export class WebSocketTransport implements Transport {
  private ws: WebSocket | null = null;
  private readonly url: string;
  private handler: ((msg: ServerEnvelope) => void) | null = null;

  constructor(url: string = 'ws://localhost:9400/ws') {
    this.url = url;
  }

  connect(): Promise<void> {
    if (this.connected) {
      return Promise.resolve();
    }

    return new Promise((resolve, reject) => {
      const ws = new WebSocket(this.url);
      let settled = false;

      ws.onopen = () => {
        this.ws = ws;

        // Protocol handshake is always sent immediately after transport open.
        this.send({
          request_id: 0,
          body: { type: 'handshake', protocol_version: '0.1.0' },
        });

        settled = true;
        resolve();
      };

      ws.onmessage = (event) => {
        try {
          const parsed = JSON.parse(String(event.data)) as ServerEnvelope;
          this.handler?.(parsed);
        } catch (error) {
          console.error('failed to parse server message', error);
        }
      };

      ws.onerror = () => {
        if (!settled) {
          settled = true;
          reject(new Error(`failed to connect websocket: ${this.url}`));
        }
      };

      ws.onclose = () => {
        if (!settled) {
          settled = true;
          reject(new Error(`websocket closed before open: ${this.url}`));
        }
        this.ws = null;
      };
    });
  }

  disconnect(): void {
    this.ws?.close();
    this.ws = null;
  }

  send(msg: ClientEnvelope): void {
    if (!this.connected || this.ws === null) {
      return;
    }

    this.ws.send(JSON.stringify(msg));
  }

  onMessage(handler: (msg: ServerEnvelope) => void): void {
    this.handler = handler;
  }

  get connected(): boolean {
    return this.ws?.readyState === WebSocket.OPEN;
  }
}
