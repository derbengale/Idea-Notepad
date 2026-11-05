import { cn } from "@/lib/utils/cn";
import { useEffect, useRef, type ReactNode } from "react";
import { Button } from "./button";

export interface DialogProps {
  open: boolean;
  title: string;
  onClose: () => void;
  children: ReactNode;
  footer?: ReactNode;
  className?: string;
  width?: string;
}

export function Dialog({ open, title, onClose, children, footer, className, width }: DialogProps) {
  const panelRef = useRef<HTMLDivElement>(null);
  const previousFocus = useRef<HTMLElement | null>(null);

  useEffect(() => {
    if (!open) {
      return;
    }
    previousFocus.current = document.activeElement as HTMLElement | null;
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.stopPropagation();
        onClose();
      }
    };
    document.addEventListener("keydown", onKeyDown);
    const timer = window.setTimeout(() => {
      const focusable = panelRef.current?.querySelector<HTMLElement>(
        "input, textarea, select, button, [tabindex]:not([tabindex='-1'])",
      );
      focusable?.focus();
    }, 20);
    return () => {
      document.removeEventListener("keydown", onKeyDown);
      window.clearTimeout(timer);
      previousFocus.current?.focus();
    };
  }, [open, onClose]);

  if (!open) {
    return null;
  }

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/45 p-6"
      onMouseDown={(event) => {
        if (event.target === event.currentTarget) {
          onClose();
        }
      }}
    >
      <div
        ref={panelRef}
        role="dialog"
        aria-modal="true"
        aria-label={title}
        style={{ width: width ?? "min(640px, 92vw)" }}
        className={cn(
          "flex max-h-[88vh] flex-col overflow-hidden rounded-[var(--r-lg)] border border-[var(--border)] bg-[var(--bg-elevated)] shadow-[var(--shadow-md)]",
          className,
        )}
      >
        <header className="flex items-center justify-between border-b border-[var(--border)] px-4 py-3">
          <h2 className="text-[15px] font-semibold">{title}</h2>
          <Button variant="ghost" size="icon" onClick={onClose} aria-label="Close">
            ✕
          </Button>
        </header>
        <div className="min-h-0 flex-1 overflow-auto px-4 py-3">{children}</div>
        {footer ? (
          <footer className="flex items-center justify-end gap-2 border-t border-[var(--border)] px-4 py-3">
            {footer}
          </footer>
        ) : null}
      </div>
    </div>
  );
}
