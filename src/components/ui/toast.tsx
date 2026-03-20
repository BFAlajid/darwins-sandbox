'use client';

import { useEffect, useRef, useState } from 'react';
import { useToastStore, type Toast } from '@/stores/toast-store';

const AUTO_DISMISS_MS = 4000;
const FADE_DURATION_MS = 300;

function ToastItem({ toast, onDismiss }: { toast: Toast; onDismiss: (id: string) => void }) {
  const [opacity, setOpacity] = useState(0);
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    // Slide/fade in on mount
    const frameId = requestAnimationFrame(() => setOpacity(1));

    // Schedule fade-out then removal
    timerRef.current = setTimeout(() => {
      setOpacity(0);
      setTimeout(() => onDismiss(toast.id), FADE_DURATION_MS);
    }, AUTO_DISMISS_MS);

    return () => {
      cancelAnimationFrame(frameId);
      if (timerRef.current !== null) {
        clearTimeout(timerRef.current);
      }
    };
  }, [toast.id, onDismiss]);

  return (
    <div
      style={{
        opacity,
        borderLeftColor: toast.color,
        transition: `opacity ${FADE_DURATION_MS}ms ease, transform ${FADE_DURATION_MS}ms ease`,
        transform: opacity === 1 ? 'translateX(0)' : 'translateX(-1rem)',
      }}
      className="flex items-center gap-2 rounded bg-black/80 px-3 py-2 text-sm text-white border-l-4 pointer-events-auto"
    >
      <span style={{ color: toast.color }} className="flex-shrink-0 text-base leading-none">
        {toast.icon}
      </span>
      <span className="leading-snug">{toast.text}</span>
    </div>
  );
}

export function ToastContainer() {
  const toasts = useToastStore((s) => s.toasts);
  const removeToast = useToastStore((s) => s.removeToast);

  if (toasts.length === 0) return null;

  return (
    <div className="absolute bottom-4 left-4 z-50 flex flex-col gap-2 pointer-events-none">
      {toasts.map((toast) => (
        <ToastItem key={toast.id} toast={toast} onDismiss={removeToast} />
      ))}
    </div>
  );
}
