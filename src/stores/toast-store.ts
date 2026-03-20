import { create } from 'zustand';

const MAX_TOASTS = 5;

export interface Toast {
  id: string;
  text: string;
  icon: string;
  color: string;
  timestamp: number;
}

interface ToastStore {
  toasts: Toast[];
  addToast: (text: string, icon: string, color: string) => void;
  removeToast: (id: string) => void;
}

export const useToastStore = create<ToastStore>((set) => ({
  toasts: [],

  addToast: (text: string, icon: string, color: string) => {
    const toast: Toast = {
      id: crypto.randomUUID(),
      text,
      icon,
      color,
      timestamp: Date.now(),
    };

    set((state) => {
      const next = [...state.toasts, toast];
      // Keep only the newest MAX_TOASTS entries
      if (next.length > MAX_TOASTS) {
        return { toasts: next.slice(next.length - MAX_TOASTS) };
      }
      return { toasts: next };
    });
  },

  removeToast: (id: string) => {
    set((state) => ({
      toasts: state.toasts.filter((t) => t.id !== id),
    }));
  },
}));
