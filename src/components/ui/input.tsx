import { cn } from "@/lib/utils/cn";
import { forwardRef, type InputHTMLAttributes, type TextareaHTMLAttributes } from "react";

const field =
  "w-full rounded-[var(--r-sm)] border border-[var(--border-strong)] bg-[var(--bg-elevated)] px-2.5 text-[13px] text-[var(--text)] placeholder:text-[var(--text-muted)] focus:border-[var(--accent)] disabled:opacity-60";

export const Input = forwardRef<HTMLInputElement, InputHTMLAttributes<HTMLInputElement>>(
  function Input({ className, ...props }, ref) {
    return <input ref={ref} className={cn(field, "h-8", className)} {...props} />;
  },
);

export const Textarea = forwardRef<
  HTMLTextAreaElement,
  TextareaHTMLAttributes<HTMLTextAreaElement>
>(function Textarea({ className, rows = 3, ...props }, ref) {
  return (
    <textarea
      ref={ref}
      rows={rows}
      className={cn(field, "py-1.5 resize-y", className)}
      {...props}
    />
  );
});
