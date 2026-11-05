import { cn } from "@/lib/utils/cn";
import type { ReactNode } from "react";

export interface BadgeProps {
  children: ReactNode;
  color?: string | null;
  className?: string;
  title?: string;
}

export function Badge({ children, color, className, title }: BadgeProps) {
  return (
    <span
      title={title}
      className={cn(
        "inline-flex items-center rounded-[var(--r-full)] px-2 py-0.5 text-[11px] font-medium leading-4",
        className,
      )}
      style={
        color
          ? {
              color,
              backgroundColor: `color-mix(in srgb, ${color} 16%, transparent)`,
              border: `1px solid color-mix(in srgb, ${color} 45%, transparent)`,
            }
          : {
              color: "var(--text-muted)",
              backgroundColor: "var(--bg-subtle)",
              border: "1px solid var(--border)",
            }
      }
    >
      {children}
    </span>
  );
}
