import { cn } from "@/lib/utils/cn";
import { forwardRef, type ButtonHTMLAttributes } from "react";

type Variant = "primary" | "secondary" | "ghost" | "danger";
type Size = "sm" | "md" | "icon";

export interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: Variant;
  size?: Size;
}

const base =
  "inline-flex items-center justify-center gap-1.5 rounded-[var(--r-sm)] font-medium transition-colors disabled:cursor-not-allowed disabled:opacity-50 whitespace-nowrap";

const variants: Record<Variant, string> = {
  primary:
    "bg-[var(--accent)] text-[var(--text-inverse)] hover:bg-[var(--accent-hover)] shadow-[var(--shadow-sm)]",
  secondary:
    "border border-[var(--border-strong)] bg-[var(--bg-elevated)] text-[var(--text)] hover:bg-[var(--bg-subtle)]",
  ghost: "text-[var(--text-muted)] hover:bg-[var(--bg-subtle)] hover:text-[var(--text)]",
  danger: "border border-[var(--danger)] text-[var(--danger)] hover:bg-[var(--danger-soft)]",
};

const sizes: Record<Size, string> = {
  sm: "h-7 px-2 text-[12px]",
  md: "h-8 px-3 text-[13px]",
  icon: "h-8 w-8",
};

export const Button = forwardRef<HTMLButtonElement, ButtonProps>(function Button(
  { className, variant = "secondary", size = "md", type = "button", ...props },
  ref,
) {
  return (
    <button
      ref={ref}
      type={type}
      className={cn(base, variants[variant], sizes[size], className)}
      {...props}
    />
  );
});
