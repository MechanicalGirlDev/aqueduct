import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type { ClientEnvelope, ServerEnvelope } from '../types';
import type { Transport } from './transport';

export class TauriTransport implements Transport {
  private handler: ((msg: ServerEnvelope) => void) | null = null;
  private unlisten: UnlistenFn | null = null;
  private _connected = false;

  async connect(): Promise<void> {
    this.unlisten = await listen<ServerEnvelope>(
      'aqueduct://server-message',
      (event) => {
        this.handler?.(event.payload);
      },
    );
    this._connected = true;

    this.send({
      request_id: 0,
      body: { type: 'handshake', protocol_version: '0.1.0' },
    });
  }

  disconnect(): void {
    this.unlisten?.();
    this.unlisten = null;
    this._connected = false;
  }

  send(msg: ClientEnvelope): void {
    if (!this._connected) {
      return;
    }

    invoke<ServerEnvelope | null>('aqueduct_dispatch', { envelope: msg })
      .then((response) => {
        if (response !== null) {
          this.handler?.(response);
        }
      })
      .catch((error) => {
        console.error('tauri invoke failed', error);
      });
  }

  onMessage(handler: (msg: ServerEnvelope) => void): void {
    this.handler = handler;
  }

  get connected(): boolean {
    return this._connected;
  }
}
