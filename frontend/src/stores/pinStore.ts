import { create } from 'zustand';
import type { PinId, PinValue } from '../types';

interface PinState {
  pinValues: Record<PinId, PinValue>;
  setPinValues: (values: Record<PinId, PinValue>) => void;
  getPinValue: (pinId: PinId) => PinValue | undefined;
}

export const usePinStore = create<PinState>((set, get) => ({
  pinValues: {},
  setPinValues: (values) => {
    set((state) => ({
      pinValues: {
        ...state.pinValues,
        ...values,
      },
    }));
  },
  getPinValue: (pinId) => get().pinValues[pinId],
}));
