import { cn } from "@/lib/utils/cn";
import { forwardRef, type SelectHTMLAttributes } from "react";

export const Select = forwardRef<HTMLSelectElement, SelectHTMLAttributes<HTMLSelectElement>>(
  function Select({ className, children, ...props }, ref) {
    return (
      <select
        ref={ref}
        className={cn(
          "h-8 w-full rounded-[var(--r-sm)] border border-[var(--border-strong)] bg-[var(--bg-elevated)] px-2 text-[13px] text-[var(--text)] focus:border-[var(--accent)]",
          className,
        )}
        {...props}
      >
        {children}
      </select>
    );
  },
);
