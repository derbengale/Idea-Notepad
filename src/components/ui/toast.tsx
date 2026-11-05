import { cn } from "@/lib/utils/cn";
import {
  createContext,
  useCallback,
  useContext,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from "react";
import { useTranslation } from "react-i18next";

export interface ToastOptions {
  tone?: "success" | "error" | "info";
  actionLabel?: string;
  onAction?: () => void;
  duration?: number;
}

interface ToastItem extends ToastOptions {
  id: number;
  message: string;
}

interface ToastContextValue {
  push: (message: string, options?: ToastOptions) => void;
}

const ToastContext = createContext<ToastContextValue | null>(null);

export function useToast(): ToastContextValue {
  const context = useContext(ToastContext);
  if (!context) {
    throw new Error("useToast must be used inside ToastProvider");
  }
  return context;
}

export function ToastProvider({ children }: { children: ReactNode }) {
  const { t } = useTranslation("common");
  const dismissLabel = t("actions.dismiss");
  const [items, setItems] = useState<ToastItem[]>([]);
  const counter = useRef(0);

  const remove = useCallback((id: number) => {
    setItems((current) => current.filter((item) => item.id !== id));
  }, []);

  const push = useCallback(
    (message: string, options?: ToastOptions) => {
      counter.current += 1;
      const id = counter.current;
      const tone = options?.tone ?? "info";
      const item: ToastItem = { id, message, tone, ...options };
      setItems((current) => [...current.slice(-3), item]);
      const duration = options?.duration ?? (tone === "error" ? 0 : 2500);
      if (duration > 0) {
        window.setTimeout(() => remove(id), duration);
      }
    },
    [remove],
  );

  const value = useMemo(() => ({ push }), [push]);

  return (
    <ToastContext.Provider value={value}>
      {children}
      <div
        aria-live="polite"
        className="pointer-events-none fixed bottom-4 right-4 z-[60] flex w-[340px] flex-col gap-2"
      >
        {items.map((item) => (
          <div
            key={item.id}
            className={cn(
              "pointer-events-auto flex items-start gap-2 rounded-[var(--r-md)] border px-3 py-2 text-[13px] shadow-[var(--shadow-md)]",
              item.tone === "error"
                ? "border-[var(--danger)] bg-[var(--danger-soft)] text-[var(--danger)]"
                : item.tone === "success"
                  ? "border-[var(--success)] bg-[var(--success-soft)] text-[var(--success)]"
                  : "border-[var(--border-strong)] bg-[var(--bg-elevated)] text-[var(--text)]",
            )}
          >
            <span className="min-w-0 flex-1 break-words">{item.message}</span>
            {item.actionLabel && item.onAction ? (
              <button
                type="button"
                className="shrink-0 font-semibold underline"
                onClick={() => {
                  item.onAction?.();
                  remove(item.id);
                }}
              >
                {item.actionLabel}
              </button>
            ) : null}
            <button
              type="button"
              className="shrink-0 opacity-60 hover:opacity-100"
              aria-label={dismissLabel}
              onClick={() => remove(item.id)}
            >
              ✕
            </button>
          </div>
        ))}
      </div>
    </ToastContext.Provider>
  );
}
