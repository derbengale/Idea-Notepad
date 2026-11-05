import { Input, Textarea } from "@/components/ui/input";
import type { FieldDef, FieldValue } from "@/lib/ipc/types";
import { cn } from "@/lib/utils/cn";
import { useTranslation } from "react-i18next";

export interface FieldFormProps {
  fields: FieldDef[];
  values: Record<string, FieldValue>;
  computed: Record<string, number | null>;
  onChange: (key: string, value: FieldValue) => void;
  disabled?: boolean;
}

export function FieldForm({ fields, values, computed, onChange, disabled }: FieldFormProps) {
  const visible = fields.filter((field) => field.type !== "tags").sort((a, b) => a.order - b.order);

  return (
    <div className="grid grid-cols-1 gap-x-6 gap-y-4 lg:grid-cols-2">
      {visible.map((field) => (
        <FieldControl
          key={field.id}
          field={field}
          value={values[field.key] ?? null}
          computed={computed[field.key]}
          onChange={(value) => onChange(field.key, value)}
          disabled={disabled}
        />
      ))}
    </div>
  );
}

function FieldControl({
  field,
  value,
  computed,
  onChange,
  disabled,
}: {
  field: FieldDef;
  value: FieldValue;
  computed: number | null | undefined;
  onChange: (value: FieldValue) => void;
  disabled?: boolean;
}) {
  const { t } = useTranslation("entries");
  const label = (
    <label className="mb-1 flex items-center gap-1 text-[12px] font-medium text-[var(--text-muted)]">
      <span>{field.label}</span>
      {field.required ? <span className="text-[var(--danger)]">*</span> : null}
    </label>
  );

  if (field.type === "formula") {
    const precision = field.config.precision ?? 2;
    const display =
      computed === null || computed === undefined
        ? "—"
        : `${computed.toFixed(precision)}${field.config.suffix ?? ""}`;
    return (
      <div className="flex flex-col">
        {label}
        <div
          className="flex h-8 items-center rounded-[var(--r-sm)] border border-dashed border-[var(--border)] bg-[var(--bg-subtle)] px-2.5 font-mono text-[13px] text-[var(--text-muted)]"
          title={t("fields.formulaHint", { expression: field.expression ?? "" })}
        >
          {display}
        </div>
      </div>
    );
  }

  if (field.type === "checkbox") {
    return (
      <div className="flex flex-col">
        {label}
        <label className="flex h-8 items-center gap-2 text-[13px]">
          <input
            type="checkbox"
            className="h-4 w-4 accent-[var(--accent)]"
            checked={value === true}
            disabled={disabled}
            onChange={(event) => onChange(event.target.checked)}
          />
          <span className="text-[var(--text-muted)]">{field.label}</span>
        </label>
      </div>
    );
  }

  if (field.type === "status") {
    const options = field.config.options ?? [];
    return (
      <div className="flex flex-col">
        {label}
        <div className="flex flex-wrap gap-1.5" role="radiogroup" aria-label={field.label}>
          {options.map((option) => {
            const active = value === option.key;
            return (
              <button
                key={option.key}
                type="button"
                role="radio"
                aria-checked={active}
                disabled={disabled}
                onClick={() => onChange(option.key)}
                className={cn(
                  "rounded-[var(--r-full)] border px-2.5 py-1 text-[12px] font-medium transition-colors",
                  active
                    ? "border-transparent text-white"
                    : "border-[var(--border-strong)] text-[var(--text-muted)] hover:bg-[var(--bg-subtle)]",
                )}
                style={active ? { backgroundColor: option.color, color: "#fff" } : undefined}
              >
                {option.label}
              </button>
            );
          })}
        </div>
      </div>
    );
  }

  if (field.type === "scale") {
    const min = field.config.min ?? 1;
    const max = field.config.max ?? 6;
    const options = Array.from({ length: max - min + 1 }, (_, index) => min + index);
    return (
      <div className="flex flex-col">
        {label}
        <div className="flex overflow-hidden rounded-[var(--r-sm)] border border-[var(--border-strong)]">
          {options.map((option) => {
            const active = value === option;
            return (
              <button
                key={option}
                type="button"
                disabled={disabled}
                aria-pressed={active}
                title={field.config.labels?.[String(option)] ?? String(option)}
                onClick={() => onChange(active ? null : option)}
                className={cn(
                  "h-8 flex-1 border-r border-[var(--border)] text-[12px] last:border-r-0",
                  active
                    ? "bg-[var(--accent)] text-[var(--text-inverse)]"
                    : "bg-[var(--bg-elevated)] text-[var(--text-muted)] hover:bg-[var(--bg-subtle)]",
                )}
              >
                {option}
              </button>
            );
          })}
        </div>
      </div>
    );
  }

  if (field.type === "date") {
    const withTime = field.config.withTime === true;
    return (
      <div className="flex flex-col">
        {label}
        <Input
          type={withTime ? "datetime-local" : "date"}
          value={typeof value === "string" ? value : ""}
          disabled={disabled}
          placeholder={withTime ? t("fields.dateTimePlaceholder") : t("fields.datePlaceholder")}
          onChange={(event) => onChange(event.target.value || null)}
        />
      </div>
    );
  }

  if (field.type === "number") {
    return (
      <div className="flex flex-col">
        {label}
        <div className="flex items-center gap-1">
          <Input
            type="number"
            value={typeof value === "number" ? value : ""}
            disabled={disabled}
            min={field.config.min}
            max={field.config.max}
            step={field.config.step ?? "any"}
            onChange={(event) =>
              onChange(event.target.value === "" ? null : Number(event.target.value))
            }
          />
          {field.config.suffix ? (
            <span className="text-[12px] text-[var(--text-muted)]">{field.config.suffix}</span>
          ) : null}
        </div>
      </div>
    );
  }

  if (field.type === "longtext") {
    return (
      <div className="col-span-full flex flex-col">
        {label}
        <Textarea
          rows={field.config.rows ?? 4}
          value={typeof value === "string" ? value : ""}
          disabled={disabled}
          placeholder={field.config.placeholder}
          onChange={(event) => onChange(event.target.value)}
        />
      </div>
    );
  }

  return (
    <div className="flex flex-col">
      {label}
      <Input
        value={typeof value === "string" ? value : ""}
        disabled={disabled}
        placeholder={field.config.placeholder}
        maxLength={field.config.maxLength}
        onChange={(event) => onChange(event.target.value)}
      />
    </div>
  );
}
