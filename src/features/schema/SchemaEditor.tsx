import { useIpcError } from "@/app/errors";
import { useUi } from "@/app/store";
import { Button } from "@/components/ui/button";
import { Dialog } from "@/components/ui/dialog";
import { Input, Textarea } from "@/components/ui/input";
import { Select } from "@/components/ui/select";
import { useToast } from "@/components/ui/toast";
import { FieldForm } from "@/features/entries/FieldForm";
import {
  formulaValidate,
  schemaCreate,
  schemaDelete,
  schemaDuplicate,
  schemaList,
  schemaUpdate,
} from "@/lib/ipc";
import type { FieldConfig, FieldDef, FieldType, SchemaDto } from "@/lib/ipc/types";
import { cn } from "@/lib/utils/cn";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { Check, ChevronDown, ChevronUp, Copy, Plus, Save, Trash2, X } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

const fieldTypes: FieldType[] = [
  "text",
  "longtext",
  "number",
  "scale",
  "status",
  "date",
  "checkbox",
  "tags",
  "formula",
];

const roleAllowed: Record<"title" | "subtitle" | "badge", FieldType[]> = {
  title: ["text"],
  subtitle: ["text", "status", "date", "number", "scale"],
  badge: ["status", "formula", "checkbox"],
};

function emptySchema(): SchemaDto {
  return {
    id: "",
    key: "",
    name: "",
    icon: "file-text",
    color: "#6366f1",
    order: 0,
    titleFieldId: null,
    subtitleFieldId: null,
    badgeFieldId: null,
    formatVersion: 1,
    fields: [],
  };
}

function newField(existing: FieldDef[]): FieldDef {
  let index = existing.length + 1;
  let key = `field_${index}`;
  while (existing.some((field) => field.key === key)) {
    index += 1;
    key = `field_${index}`;
  }
  return {
    id: crypto.randomUUID(),
    key,
    label: "Field",
    type: "text",
    required: false,
    order: existing.length,
    config: {},
    expression: null,
    showInList: true,
  };
}

export function SchemaEditor() {
  const { t } = useTranslation(["schema", "common"]);
  const queryClient = useQueryClient();
  const showError = useIpcError();
  const { push } = useToast();
  const { schemaEditorId, schemaEditorNew, closeSchemaEditor, openSchemaEditor } = useUi();
  const schemasQuery = useQuery({ queryKey: ["schemas"], queryFn: schemaList });
  const schemas = useMemo(() => schemasQuery.data ?? [], [schemasQuery.data]);

  const [draft, setDraft] = useState<SchemaDto>(emptySchema());
  const [selectedFieldId, setSelectedFieldId] = useState<string | null>(null);
  const [deleteOpen, setDeleteOpen] = useState(false);
  const [deleteTarget, setDeleteTarget] = useState("");
  const [validation, setValidation] = useState<{ ok: boolean; error: string | null } | null>(null);
  const [previewOpen, setPreviewOpen] = useState(false);

  const isNew = schemaEditorNew || schemaEditorId === null;
  const isOpen = schemaEditorNew || schemaEditorId !== null;

  const source = useMemo(
    () => (isNew ? emptySchema() : schemas.find((schema) => schema.id === schemaEditorId)),
    [isNew, schemaEditorId, schemas],
  );

  useEffect(() => {
    if (!isOpen) {
      return;
    }
    if (source) {
      setDraft(structuredClone(source));
      setSelectedFieldId(source.fields[0]?.id ?? null);
    } else if (isNew && schemasQuery.isFetched) {
      setDraft(emptySchema());
      setSelectedFieldId(null);
    }
  }, [isOpen, source, isNew, schemasQuery.isFetched]);

  const selectedField = draft.fields.find((field) => field.id === selectedFieldId) ?? null;
  const listColumns = draft.fields.filter((field) => field.showInList).length;

  if (!isOpen) {
    return null;
  }

  const updateField = (id: string, patch: Partial<FieldDef>) => {
    setDraft((current) => ({
      ...current,
      fields: current.fields.map((field) => (field.id === id ? { ...field, ...patch } : field)),
    }));
  };

  const updateConfig = (id: string, patch: FieldConfig) => {
    setDraft((current) => ({
      ...current,
      fields: current.fields.map((field) =>
        field.id === id ? { ...field, config: { ...field.config, ...patch } } : field,
      ),
    }));
  };

  const moveField = (id: string, direction: -1 | 1) => {
    setDraft((current) => {
      const fields = [...current.fields].sort((a, b) => a.order - b.order);
      const index = fields.findIndex((field) => field.id === id);
      const target = index + direction;
      if (index < 0 || target < 0 || target >= fields.length) {
        return current;
      }
      const swapped = fields[index];
      const other = fields[target];
      if (!swapped || !other) {
        return current;
      }
      fields[index] = other;
      fields[target] = swapped;
      return {
        ...current,
        fields: fields.map((field, position) => ({ ...field, order: position })),
      };
    });
  };

  const save = async () => {
    try {
      const payload: SchemaDto = {
        ...draft,
        fields: draft.fields.map((field, index) => ({ ...field, order: index })),
      };
      const saved = isNew
        ? await schemaCreate({
            ...payload,
            id: undefined,
            key: payload.key || undefined,
          })
        : await schemaUpdate(payload);
      void queryClient.invalidateQueries({ queryKey: ["schemas"] });
      push(t("schema:editor.saved"), { tone: "success" });
      if (isNew) {
        openSchemaEditor(saved.id);
      } else {
        setDraft(structuredClone(saved));
      }
    } catch (error) {
      showError(error);
    }
  };

  const removeField = (field: FieldDef) => {
    setDraft((current) => ({
      ...current,
      fields: current.fields
        .filter((item) => item.id !== field.id)
        .map((item, index) => ({ ...item, order: index })),
      titleFieldId: current.titleFieldId === field.id ? null : current.titleFieldId,
      subtitleFieldId: current.subtitleFieldId === field.id ? null : current.subtitleFieldId,
      badgeFieldId: current.badgeFieldId === field.id ? null : current.badgeFieldId,
    }));
    if (selectedFieldId === field.id) {
      setSelectedFieldId(null);
    }
  };

  const handleDelete = async () => {
    if (!draft.id) {
      closeSchemaEditor();
      return;
    }
    if (!deleteTarget) {
      return;
    }
    try {
      await schemaDelete(draft.id, deleteTarget);
      setDeleteOpen(false);
      closeSchemaEditor();
      void queryClient.invalidateQueries();
    } catch (error) {
      showError(error);
    }
  };

  const handleDuplicate = async () => {
    if (!draft.id) {
      return;
    }
    try {
      const copy = await schemaDuplicate(draft.id, `${draft.name} copy`);
      void queryClient.invalidateQueries({ queryKey: ["schemas"] });
      openSchemaEditor(copy.id);
    } catch (error) {
      showError(error);
    }
  };

  const validateFormula = async () => {
    if (!selectedField?.expression) {
      return;
    }
    const result = await formulaValidate(selectedField.expression, draft.id || undefined);
    setValidation({ ok: result.ok, error: result.error });
  };

  return (
    <div className="fixed inset-0 z-40 flex flex-col bg-[var(--bg)]">
      <header className="flex items-center justify-between border-b border-[var(--border)] bg-[var(--bg-elevated)] px-4 py-2">
        <h1 className="text-[14px] font-semibold">{t("schema:editor.title")}</h1>
        <div className="flex items-center gap-1">
          {!isNew ? (
            <Button variant="ghost" size="sm" onClick={() => void handleDuplicate()}>
              <Copy size={13} />
              {t("schema:editor.duplicate")}
            </Button>
          ) : null}
          <Button
            variant="ghost"
            size="sm"
            className="text-[var(--danger)]"
            disabled={isNew || schemas.length <= 1}
            onClick={() => {
              setDeleteTarget(schemas.find((schema) => schema.id !== draft.id)?.id ?? "");
              setDeleteOpen(true);
            }}
          >
            <Trash2 size={13} />
            {t("schema:editor.delete")}
          </Button>
          <Button variant="primary" size="sm" onClick={() => void save()}>
            <Save size={13} />
            {t("schema:editor.save")}
          </Button>
          <Button
            variant="ghost"
            size="icon"
            onClick={closeSchemaEditor}
            aria-label={t("common:actions.close")}
          >
            <X size={16} />
          </Button>
        </div>
      </header>

      <div className="grid min-h-0 flex-1 grid-cols-[190px_230px_1fr]">
        <aside className="min-h-0 overflow-auto border-r border-[var(--border)] bg-[var(--bg-elevated)] p-2">
          <Button
            variant="secondary"
            size="sm"
            className="mb-2 w-full"
            onClick={() => {
              openSchemaEditor(null);
              setDraft(emptySchema());
              setSelectedFieldId(null);
            }}
          >
            <Plus size={13} />
            {t("schema:editor.new")}
          </Button>
          {schemas.map((schema) => (
            <button
              key={schema.id}
              type="button"
              onClick={() => openSchemaEditor(schema.id)}
              className={cn(
                "mb-0.5 flex w-full items-center gap-1.5 truncate rounded-[var(--r-sm)] px-2 py-1.5 text-left text-[12.5px]",
                schema.id === schemaEditorId
                  ? "bg-[var(--accent-soft)] font-medium text-[var(--accent)]"
                  : "hover:bg-[var(--bg-subtle)]",
              )}
            >
              <span
                className="h-2 w-2 shrink-0 rounded-full"
                style={{ backgroundColor: schema.color }}
              />
              {schema.name}
            </button>
          ))}
        </aside>

        <section className="min-h-0 overflow-auto border-r border-[var(--border)] p-3">
          <div className="space-y-2">
            <label className="block">
              <span className="text-[11px] text-[var(--text-muted)]">
                {t("schema:editor.name")}
              </span>
              <Input
                value={draft.name}
                onChange={(event) => setDraft({ ...draft, name: event.target.value })}
                placeholder={t("schema:editor.name")}
              />
            </label>
            <div className="grid grid-cols-2 gap-2">
              <label className="block">
                <span className="text-[11px] text-[var(--text-muted)]">
                  {t("schema:editor.icon")}
                </span>
                <Input
                  value={draft.icon}
                  onChange={(event) => setDraft({ ...draft, icon: event.target.value })}
                />
              </label>
              <label className="block">
                <span className="text-[11px] text-[var(--text-muted)]">
                  {t("schema:editor.color")}
                </span>
                <input
                  type="color"
                  value={draft.color}
                  onChange={(event) => setDraft({ ...draft, color: event.target.value })}
                  className="h-8 w-full rounded-[var(--r-sm)] border border-[var(--border-strong)] bg-[var(--bg-elevated)] p-0.5"
                />
              </label>
            </div>
            {isNew && draft.key ? (
              <div className="text-[11px] text-[var(--text-muted)]">
                {t("schema:editor.fieldKey")}: <code>{draft.key}</code>
              </div>
            ) : null}
            <div className="text-[11px] text-[var(--text-muted)]">
              {t("schema:editor.listColumns", { count: listColumns })}
            </div>
          </div>

          <div className="mt-4 flex items-center justify-between">
            <h2 className="text-[11px] font-semibold uppercase tracking-wide text-[var(--text-muted)]">
              {t("schema:editor.fields")}
            </h2>
            <Button
              size="sm"
              variant="ghost"
              className="h-6 px-1.5 text-[11px]"
              onClick={() => {
                const field = newField(draft.fields);
                setDraft({ ...draft, fields: [...draft.fields, field] });
                setSelectedFieldId(field.id);
              }}
            >
              <Plus size={12} />
              {t("schema:editor.addField")}
            </Button>
          </div>

          <ul className="mt-1 space-y-0.5">
            {[...draft.fields]
              .sort((a, b) => a.order - b.order)
              .map((field, index) => (
                <li key={field.id}>
                  <div
                    className={cn(
                      "group flex items-center gap-1 rounded-[var(--r-sm)] px-1.5 py-1",
                      field.id === selectedFieldId
                        ? "bg-[var(--accent-soft)]"
                        : "hover:bg-[var(--bg-subtle)]",
                    )}
                  >
                    <button
                      type="button"
                      className="min-w-0 flex-1 text-left"
                      onClick={() => setSelectedFieldId(field.id)}
                    >
                      <span className="block truncate text-[12px]">{field.label}</span>
                      <span className="block text-[10.5px] text-[var(--text-muted)]">
                        {t(`schema:type.${field.type}`)} · {field.key}
                      </span>
                    </button>
                    <button
                      type="button"
                      className="opacity-0 group-hover:opacity-100 disabled:opacity-20"
                      title={t("schema:editor.moveUp")}
                      aria-label={t("schema:editor.moveUp")}
                      disabled={index === 0}
                      onClick={() => moveField(field.id, -1)}
                    >
                      <ChevronUp size={12} />
                    </button>
                    <button
                      type="button"
                      className="opacity-0 group-hover:opacity-100 disabled:opacity-20"
                      title={t("schema:editor.moveDown")}
                      aria-label={t("schema:editor.moveDown")}
                      disabled={index === draft.fields.length - 1}
                      onClick={() => moveField(field.id, 1)}
                    >
                      <ChevronDown size={12} />
                    </button>
                  </div>
                </li>
              ))}
          </ul>
          {draft.fields.length === 0 ? (
            <p className="mt-2 text-[11.5px] text-[var(--text-muted)]">
              {t("schema:editor.noFields")}
            </p>
          ) : null}
        </section>

        <section className="min-h-0 overflow-auto p-4">
          {selectedField ? (
            <FieldInspector
              key={selectedField.id}
              schema={draft}
              field={selectedField}
              isExisting={
                !isNew && Boolean(source?.fields.some((field) => field.id === selectedField.id))
              }
              onUpdate={(patch) => updateField(selectedField.id, patch)}
              onConfig={(patch) => updateConfig(selectedField.id, patch)}
              onRemove={() => removeField(selectedField)}
              validation={validation}
              onValidate={() => void validateFormula()}
              onRoleChange={(role, value) =>
                setDraft({
                  ...draft,
                  ...(role === "title"
                    ? { titleFieldId: value }
                    : role === "subtitle"
                      ? { subtitleFieldId: value }
                      : { badgeFieldId: value }),
                })
              }
            />
          ) : (
            <p className="text-[12.5px] text-[var(--text-muted)]">{t("schema:editor.noFields")}</p>
          )}

          <div className="mt-6 border-t border-[var(--border)] pt-3">
            <button
              type="button"
              className="mb-2 text-[11px] font-semibold uppercase tracking-wide text-[var(--text-muted)]"
              onClick={() => setPreviewOpen((current) => !current)}
            >
              {t("schema:editor.preview")}
            </button>
            {previewOpen ? (
              <FieldForm
                fields={draft.fields}
                values={{}}
                computed={{}}
                onChange={() => undefined}
                disabled
              />
            ) : null}
          </div>
        </section>
      </div>

      <Dialog
        open={deleteOpen}
        title={t("schema:editor.deleteTitle")}
        onClose={() => setDeleteOpen(false)}
        footer={
          <>
            <Button onClick={() => setDeleteOpen(false)}>{t("common:actions.cancel")}</Button>
            <Button variant="danger" onClick={() => void handleDelete()} disabled={!deleteTarget}>
              {t("schema:editor.deleteConfirm")}
            </Button>
          </>
        }
      >
        <p className="mb-3 text-[13px]">{t("schema:editor.deleteMessage", { count: 0 })}</p>
        <label className="block">
          <span className="text-[12px] text-[var(--text-muted)]">
            {t("schema:editor.targetSchema")}
          </span>
          <Select value={deleteTarget} onChange={(event) => setDeleteTarget(event.target.value)}>
            {schemas
              .filter((schema) => schema.id !== draft.id)
              .map((schema) => (
                <option key={schema.id} value={schema.id}>
                  {schema.name}
                </option>
              ))}
          </Select>
        </label>
      </Dialog>
    </div>
  );
}

function FieldInspector({
  schema,
  field,
  isExisting,
  onUpdate,
  onConfig,
  onRemove,
  validation,
  onValidate,
  onRoleChange,
}: {
  schema: SchemaDto;
  field: FieldDef;
  isExisting: boolean;
  onUpdate: (patch: Partial<FieldDef>) => void;
  onConfig: (patch: FieldConfig) => void;
  onRemove: () => void;
  validation: { ok: boolean; error: string | null } | null;
  onValidate: () => void;
  onRoleChange: (role: "title" | "subtitle" | "badge", value: string | null) => void;
}) {
  const { t } = useTranslation(["schema", "common"]);

  return (
    <div className="space-y-3">
      <div className="flex items-center justify-between">
        <h2 className="text-[12px] font-semibold uppercase tracking-wide text-[var(--text-muted)]">
          {field.label}
        </h2>
        <Button
          variant="ghost"
          size="sm"
          className="text-[var(--danger)]"
          onClick={onRemove}
          title={t("schema:editor.removeField")}
        >
          <Trash2 size={12} />
          {t("schema:editor.removeField")}
        </Button>
      </div>

      <label className="block">
        <span className="text-[11px] text-[var(--text-muted)]">
          {t("schema:editor.fieldLabel")}
        </span>
        <Input value={field.label} onChange={(event) => onUpdate({ label: event.target.value })} />
      </label>

      <label className="block">
        <span className="text-[11px] text-[var(--text-muted)]">{t("schema:editor.fieldKey")}</span>
        <Input
          value={field.key}
          disabled={isExisting}
          title={isExisting ? t("schema:editor.keyLockedHint") : undefined}
          onChange={(event) => onUpdate({ key: event.target.value })}
        />
        {isExisting ? (
          <span className="text-[10.5px] text-[var(--text-muted)]">
            {t("schema:editor.keyLocked")}
          </span>
        ) : null}
      </label>

      <div className="grid grid-cols-2 gap-2">
        <label className="block">
          <span className="text-[11px] text-[var(--text-muted)]">
            {t("schema:editor.fieldType")}
          </span>
          <Select
            value={field.type}
            onChange={(event) => {
              const type = event.target.value as FieldType;
              onUpdate({ type, expression: type === "formula" ? field.expression : null });
            }}
          >
            {fieldTypes.map((type) => (
              <option key={type} value={type}>
                {t(`schema:type.${type}`)}
              </option>
            ))}
          </Select>
        </label>
        <div className="space-y-1 pt-4">
          <label className="flex items-center gap-1.5 text-[12px]">
            <input
              type="checkbox"
              className="h-3.5 w-3.5 accent-[var(--accent)]"
              checked={field.required}
              disabled={field.type === "formula" || field.type === "tags"}
              onChange={(event) => onUpdate({ required: event.target.checked })}
            />
            {t("schema:editor.required")}
          </label>
          <label className="flex items-center gap-1.5 text-[12px]">
            <input
              type="checkbox"
              className="h-3.5 w-3.5 accent-[var(--accent)]"
              checked={field.showInList}
              onChange={(event) => onUpdate({ showInList: event.target.checked })}
            />
            {t("schema:editor.showInList")}
          </label>
        </div>
      </div>

      <div className="grid grid-cols-3 gap-2">
        {(["title", "subtitle", "badge"] as const).map((roleName) => {
          const value =
            roleName === "title"
              ? schema.titleFieldId
              : roleName === "subtitle"
                ? schema.subtitleFieldId
                : schema.badgeFieldId;
          const allowed = roleAllowed[roleName].includes(field.type);
          return (
            <label key={roleName} className="block">
              <span className="text-[11px] text-[var(--text-muted)]">
                {t(`schema:editor.${roleName}Field`)}
              </span>
              <Select
                value={value === field.id ? field.id : ""}
                disabled={!allowed}
                onChange={(event) => onRoleChange(roleName, event.target.value || null)}
              >
                <option value="">{t("schema:editor.none")}</option>
                <option value={field.id}>{field.label}</option>
              </Select>
            </label>
          );
        })}
      </div>

      <div className="space-y-2 border-t border-[var(--border)] pt-3">
        <h3 className="text-[11px] font-semibold uppercase tracking-wide text-[var(--text-muted)]">
          {t("schema:editor.config")}
        </h3>
        {field.type === "text" || field.type === "longtext" ? (
          <>
            <NumberConfig
              label={t("schema:config.maxLength")}
              value={field.config.maxLength}
              onChange={(maxLength) => onConfig({ maxLength })}
            />
            {field.type === "longtext" ? (
              <NumberConfig
                label={t("schema:config.rows")}
                value={field.config.rows}
                onChange={(rows) => onConfig({ rows })}
              />
            ) : null}
          </>
        ) : null}
        {field.type === "number" ? (
          <div className="grid grid-cols-2 gap-2">
            <NumberConfig
              label={t("schema:config.min")}
              value={field.config.min}
              onChange={(min) => onConfig({ min })}
            />
            <NumberConfig
              label={t("schema:config.max")}
              value={field.config.max}
              onChange={(max) => onConfig({ max })}
            />
            <NumberConfig
              label={t("schema:config.step")}
              value={field.config.step}
              onChange={(step) => onConfig({ step })}
            />
            <NumberConfig
              label={t("schema:config.precision")}
              value={field.config.precision}
              onChange={(precision) => onConfig({ precision })}
            />
            <label className="block">
              <span className="text-[11px] text-[var(--text-muted)]">
                {t("schema:config.suffix")}
              </span>
              <Input
                value={field.config.suffix ?? ""}
                onChange={(event) => onConfig({ suffix: event.target.value })}
              />
            </label>
          </div>
        ) : null}
        {field.type === "scale" ? (
          <div className="grid grid-cols-2 gap-2">
            <NumberConfig
              label={t("schema:config.min")}
              value={field.config.min}
              onChange={(min) => onConfig({ min })}
            />
            <NumberConfig
              label={t("schema:config.max")}
              value={field.config.max}
              onChange={(max) => onConfig({ max })}
            />
          </div>
        ) : null}
        {field.type === "date" ? (
          <label className="flex items-center gap-2 text-[12px]">
            <input
              type="checkbox"
              className="h-3.5 w-3.5 accent-[var(--accent)]"
              checked={field.config.withTime === true}
              onChange={(event) => onConfig({ withTime: event.target.checked })}
            />
            {t("schema:config.withTime")}
          </label>
        ) : null}
        {field.type === "checkbox" ? (
          <label className="flex items-center gap-2 text-[12px]">
            <input
              type="checkbox"
              className="h-3.5 w-3.5 accent-[var(--accent)]"
              checked={field.config.default === true}
              onChange={(event) => onConfig({ default: event.target.checked })}
            />
            {t("schema:config.defaultValue")}
          </label>
        ) : null}
        {field.type === "status" ? (
          <div className="space-y-1.5">
            <span className="text-[11px] text-[var(--text-muted)]">
              {t("schema:config.options")}
            </span>
            {(field.config.options ?? []).map((option, index) => (
              <div key={index} className="flex items-center gap-1">
                <Input
                  className="w-[86px]"
                  value={option.key}
                  aria-label={t("schema:config.optionKey")}
                  onChange={(event) => {
                    const options = [...(field.config.options ?? [])];
                    options[index] = { ...option, key: event.target.value };
                    onConfig({ options });
                  }}
                />
                <Input
                  value={option.label}
                  aria-label={t("schema:config.optionLabel")}
                  onChange={(event) => {
                    const options = [...(field.config.options ?? [])];
                    options[index] = { ...option, label: event.target.value };
                    onConfig({ options });
                  }}
                />
                <input
                  type="color"
                  value={option.color}
                  aria-label={t("schema:config.optionColor")}
                  onChange={(event) => {
                    const options = [...(field.config.options ?? [])];
                    options[index] = { ...option, color: event.target.value };
                    onConfig({ options });
                  }}
                  className="h-8 w-8 shrink-0 rounded border border-[var(--border-strong)] bg-transparent p-0.5"
                />
                <button
                  type="button"
                  aria-label={t("schema:config.removeOption")}
                  onClick={() =>
                    onConfig({
                      options: (field.config.options ?? []).filter(
                        (_, position) => position !== index,
                      ),
                    })
                  }
                >
                  <X size={12} />
                </button>
              </div>
            ))}
            <Button
              size="sm"
              variant="ghost"
              onClick={() =>
                onConfig({
                  options: [
                    ...(field.config.options ?? []),
                    {
                      key: `option_${(field.config.options?.length ?? 0) + 1}`,
                      label: "Option",
                      color: "#64748b",
                    },
                  ],
                })
              }
            >
              <Plus size={12} />
              {t("schema:config.addOption")}
            </Button>
          </div>
        ) : null}
        {field.type === "formula" ? (
          <div className="space-y-1.5">
            <span className="text-[11px] text-[var(--text-muted)]">
              {t("schema:editor.expression")}
            </span>
            <Textarea
              className="font-mono text-[12px]"
              rows={3}
              value={field.expression ?? ""}
              onChange={(event) => onUpdate({ expression: event.target.value })}
            />
            <div className="flex items-center gap-2">
              <Button size="sm" onClick={onValidate}>
                {t("schema:editor.validate")}
              </Button>
              {validation ? (
                <span
                  className={cn(
                    "flex items-center gap-1 text-[11.5px]",
                    validation.ok ? "text-[var(--success)]" : "text-[var(--danger)]",
                  )}
                >
                  {validation.ok ? <Check size={12} /> : <X size={12} />}
                  {validation.ok
                    ? t("schema:editor.valid")
                    : `${t("schema:editor.invalid")}: ${validation.error ?? ""}`}
                </span>
              ) : null}
            </div>
            <div className="grid grid-cols-2 gap-2">
              <NumberConfig
                label={t("schema:config.precision")}
                value={field.config.precision}
                onChange={(precision) => onConfig({ precision })}
              />
              <label className="block">
                <span className="text-[11px] text-[var(--text-muted)]">
                  {t("schema:config.suffix")}
                </span>
                <Input
                  value={field.config.suffix ?? ""}
                  onChange={(event) => onConfig({ suffix: event.target.value })}
                />
              </label>
            </div>
          </div>
        ) : null}
        {field.type === "tags" ? (
          <p className="text-[11.5px] text-[var(--text-muted)]">
            {t("schema:editor.fieldKey")}: <code>tags</code>
          </p>
        ) : null}
      </div>

      <p className="text-[10.5px] text-[var(--text-muted)]">
        {isExisting ? t("schema:editor.keyLockedHint") : t("schema:editor.invalidKey")}
      </p>
    </div>
  );
}

function NumberConfig({
  label,
  value,
  onChange,
}: {
  label: string;
  value: number | undefined;
  onChange: (value: number | undefined) => void;
}) {
  return (
    <label className="block">
      <span className="text-[11px] text-[var(--text-muted)]">{label}</span>
      <Input
        type="number"
        value={value ?? ""}
        onChange={(event) =>
          onChange(event.target.value === "" ? undefined : Number(event.target.value))
        }
      />
    </label>
  );
}
