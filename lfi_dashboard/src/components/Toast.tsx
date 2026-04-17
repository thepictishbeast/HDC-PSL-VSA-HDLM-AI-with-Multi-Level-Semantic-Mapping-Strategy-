import React, { useEffect, useState } from 'react';
import { T } from '../tokens';

export type ToastType = 'error' | 'success' | 'warning' | 'info';

export interface ToastMessage {
  id: string;
  type: ToastType;
  message: string;
  duration?: number;
}

interface ToastProps {
  C: any;
  toast: ToastMessage;
  onDismiss: (id: string) => void;
}

const ToastItem: React.FC<ToastProps> = ({ C, toast, onDismiss }) => {
  const [exiting, setExiting] = useState(false);

  useEffect(() => {
    const dur = toast.duration ?? 5000;
    const timer = setTimeout(() => {
      setExiting(true);
      setTimeout(() => onDismiss(toast.id), 300);
    }, dur);
    return () => clearTimeout(timer);
  }, [toast.id, toast.duration, onDismiss]);

  const colors: Record<ToastType, { bg: string; border: string; text: string }> = {
    error: { bg: C.redBg || '#2d1515', border: C.redBorder || '#7f1d1d', text: C.red || '#ef4444' },
    success: { bg: '#0d2818', border: '#166534', text: '#22c55e' },
    warning: { bg: '#2d2200', border: '#854d0e', text: '#eab308' },
    info: { bg: C.bgHover || '#1a1a2e', border: C.border || '#334155', text: C.textSecondary || '#94a3b8' },
  };

  const c = colors[toast.type];

  return (
    <div
      role="alert"
      style={{
        padding: `${T.spacing.sm} ${T.spacing.md}`,
        background: c.bg,
        border: `1px solid ${c.border}`,
        color: c.text,
        borderRadius: T.radii.md,
        fontSize: T.typography.sizeSm,
        display: 'flex',
        alignItems: 'center',
        gap: T.spacing.sm,
        boxShadow: '0 4px 12px rgba(0,0,0,0.3)',
        transform: exiting ? 'translateX(120%)' : 'translateX(0)',
        opacity: exiting ? 0 : 1,
        transition: 'transform 0.3s ease, opacity 0.3s ease',
        maxWidth: '400px',
        pointerEvents: 'auto',
      }}
    >
      <span style={{ flex: 1 }}>{toast.message}</span>
      <button
        onClick={() => { setExiting(true); setTimeout(() => onDismiss(toast.id), 300); }}
        style={{
          background: 'transparent',
          border: 'none',
          color: c.text,
          cursor: 'pointer',
          fontSize: '16px',
          padding: '2px 6px',
          lineHeight: 1,
          opacity: 0.7,
        }}
        aria-label="Dismiss"
      >
        ×
      </button>
    </div>
  );
};

interface ToastContainerProps {
  C: any;
  toasts: ToastMessage[];
  onDismiss: (id: string) => void;
}

export const ToastContainer: React.FC<ToastContainerProps> = ({ C, toasts, onDismiss }) => (
  <div
    style={{
      position: 'fixed',
      top: T.spacing.lg,
      right: T.spacing.lg,
      zIndex: 9999,
      display: 'flex',
      flexDirection: 'column',
      gap: T.spacing.sm,
      pointerEvents: 'none',
    }}
  >
    {toasts.map((t) => (
      <ToastItem key={t.id} C={C} toast={t} onDismiss={onDismiss} />
    ))}
  </div>
);

// Hook for managing toasts
let toastCounter = 0;
export function useToasts() {
  const [toasts, setToasts] = useState<ToastMessage[]>([]);

  const addToast = (type: ToastType, message: string, duration?: number) => {
    const id = `toast-${++toastCounter}-${Date.now()}`;
    setToasts((prev) => [...prev.slice(-4), { id, type, message, duration }]);
  };

  const dismissToast = (id: string) => {
    setToasts((prev) => prev.filter((t) => t.id !== id));
  };

  return {
    toasts,
    addToast,
    dismissToast,
    error: (msg: string) => addToast('error', msg),
    success: (msg: string) => addToast('success', msg),
    warning: (msg: string) => addToast('warning', msg),
    info: (msg: string) => addToast('info', msg),
  };
}
