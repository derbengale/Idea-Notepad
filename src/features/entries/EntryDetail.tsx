import { useIpcError } from "@/app/errors";
import { useUi } from "@/app/store";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { ContextMenu, useContextMenu, type ContextMenuEntry } from "@/components/ui/context-menu";
import { Dialog } from "@/components/ui/dialog";
import { Select } from "@/components/ui/select";
import { useToast } from "@/components/ui/toast";
import { MarkdownEditor } from "@/features/editor/MarkdownEditor";
import { MarkdownPreview } from "@/features/editor/MarkdownPreview";
import { SketchGallery } from "@/features/sketch/SketchGallery";
import {
    attachmentAdd,
    attachmentDelete,
    attachmentOpen,
    entryDelete,
    entryDuplicate,
    entryGet,
    entryList,
    entryRestore,
    entryUpdate,
    exportEntryMarkdown,
    linkCreate,
    linkDelete,
    sketchCreate,
    sketchDelete,
    tagCreate,
} from "@/lib/ipc";
import type {
    AppInfo,
    AttachmentDto,
    EntryDetail as EntryDetailDto,
    EntryListItem,
    FieldValue,
    ProjectDto,
    SchemaDto,
    Settings,
    TagDto,
} from "@/lib/ipc/types";
import { cn } from "@/lib/utils/cn";
import { formatBytes, formatDate, formatRelative, wordCount } from "@/lib/utils/format";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { convertFileSrc } from "@tauri-apps/api/core";
import { open as openDialog, save as saveDialog } from "@tauri-apps/plugin-dialog";
import {
    ArrowRightLeft,
    Check,
    Copy,
    Download,
    FileText,
    ImagePlus,
    Link2,
    Loader2,
    Paperclip,
    Pencil,
    Plus,
    RotateCcw,
    Trash2,
    X,
} from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { FieldForm } from "./FieldForm";

interface Draft {
  title: string;
  bodyMd: string;
  values: Record<string, FieldValue>;
  projectId: string | null;
  tagIds: string[];
}

function toDraft(entry: EntryDetailDto): Draft {
  return {
    title: entry.title,
    bodyMd: entry.bodyMd,
    values: { ...entry.values },
    projectId: entry.projectId,
    tagIds: entry.tags.map((tag) => tag.id),
  };
}

export interface EntryDetailProps {
  entryId: string;
  schemas: SchemaDto[];
  projects: ProjectDto[];
  tags: TagDto[];
  appInfo: AppInfo;
  settings: Settings;
}

export function EntryDetail({
  entryId,
  schemas,
  projects,
  tags,
  appInfo,
  settings,
}: EntryDetailProps) {
  const { t, i18n } = useTranslation(["entries", "common", "editor"]);
  const queryClient = useQueryClient();
  const showError = useIpcError();
  const { push } = useToast();
  const navigateEntry = useUi((state) => state.navigateEntry);
  const selectEntry = useUi((state) => state.selectEntry);
  const editorMode = useUi((state) => state.editorMode);
  const setEditorMode = useUi((state) => state.setEditorMode);
  const setSketchId = useUi((state) => state.setSketchId);
  const editRequestId = useUi((state) => state.editRequestId);
  const clearEditRequest = useUi((state) => state.clearEditRequest);

  const entryQuery = useQuery({
    queryKey: ["entry", entryId],
    queryFn: () => entryGet(entryId),
  });
  const allEntries = useQuery({
    queryKey: ["entries", "all"],
    queryFn: () => entryList({ limit: 500 }),
  });

  const entry = entryQuery.data;
  const schema = useMemo(
    () => schemas.find((candidate) => candidate.id === entry?.schemaId) ?? null,
    [schemas, entry?.schemaId],
  );

  const [draft, setDraft] = useState<Draft | null>(null);
  const [status, setStatus] = useState<"idle" | "saving" | "saved" | "error">("idle");
  const [deleteOpen, setDeleteOpen] = useState(false);
  const [deleteAttachmentId, setDeleteAttachmentId] = useState<string | null>(null);
  const [editing, setEditing] = useState(false);
  const [exporting, setExporting] = useState(false);
  const dirty = useRef(false);
  const saving = useRef(false);
  const queued = useRef(false);
  const baseline = useRef<string | null>(null);
  const draftRef = useRef<Draft | null>(null);
  const entryRef = useRef<EntryDetailDto | undefined>(undefined);
  draftRef.current = draft;
  entryRef.current = entry;

  useEffect(() => {
    if (entry && entry.id === entryId) {
      setDraft(toDraft(entry));
      baseline.current = entry.updatedAt;
      dirty.current = false;
      setStatus("idle");
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [entryId, entry?.id]);

  useEffect(() => {
    if (!entry || entry.id !== entryId || dirty.current) {
      return;
    }
    setDraft((current) =>
      current && current.projectId !== entry.projectId
        ? { ...current, projectId: entry.projectId }
        : current,
    );
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [entryId, entry?.id, entry?.projectId]);

  useEffect(() => {
    if (editRequestId === entryId) {
      setEditing(true);
      clearEditRequest();
    }
  }, [editRequestId, entryId, clearEditRequest]);

  const save = useCallback(async () => {
    const current = draftRef.current;
    const currentEntry = entryRef.current;
    if (!current || !currentEntry || currentEntry.id !== entryId) {
      return;
    }
    if (!current.title.trim()) {
      return;
    }
    if (saving.current) {
      queued.current = true;
      return;
    }
    saving.current = true;
    const snapshot = current;
    setStatus("saving");
    try {
      const updated = await entryUpdate({
        id: entryId,
        schemaId: currentEntry.schemaId,
        projectId: current.projectId,
        title: current.title.trim(),
        bodyMd: current.bodyMd,
        values: current.values,
        tagIds: current.tagIds,
        links: currentEntry.links.links.map((link) => link.entryId),
        expectedUpdatedAt: baseline.current,
      });
      queryClient.setQueryData(["entry", entryId], updated);
      baseline.current = updated.updatedAt;
      if (draftRef.current === snapshot) {
        dirty.current = false;
        setStatus("saved");
      }
      void queryClient.invalidateQueries({ queryKey: ["entries"] });
      void queryClient.invalidateQueries({ queryKey: ["stats"] });
    } catch (error) {
      setStatus("error");
      showError(error);
    } finally {
      saving.current = false;
      if (queued.current) {
        queued.current = false;
        void save();
      }
    }
  }, [entryId, queryClient, showError]);

  useEffect(() => {
    if (!dirty.current) {
      return;
    }
    const timer = window.setTimeout(() => void save(), 600);
    return () => window.clearTimeout(timer);
  }, [draft, save]);

  const updateDraft = useCallback((patch: Partial<Draft>) => {
    dirty.current = true;
    setDraft((current) => (current ? { ...current, ...patch } : current));
  }, []);

  const setValue = useCallback((key: string, value: FieldValue) => {
    setDraft((current) => {
      if (!current) {
        return current;
      }
      dirty.current = true;
      return { ...current, values: { ...current.values, [key]: value } };
    });
  }, []);

  const invalidateEntry = useCallback(() => {
    void queryClient.invalidateQueries({ queryKey: ["entry", entryId] });
    void queryClient.invalidateQueries({ queryKey: ["entries"] });
  }, [entryId, queryClient]);

  const createTag = useMutation({
    mutationFn: (name: string) => tagCreate(name),
    onSuccess: (tag) => {
      void queryClient.invalidateQueries({ queryKey: ["tags"] });
      if (draftRef.current && !draftRef.current.tagIds.includes(tag.id)) {
        updateDraft({ tagIds: [...draftRef.current.tagIds, tag.id] });
      }
    },
    onError: showError,
  });

  const handleAddLink = async (targetId: string) => {
    if (!targetId) {
      return;
    }
    try {
      await linkCreate(entryId, targetId);
      invalidateEntry();
    } catch (error) {
      showError(error);
    }
  };

  const handleRemoveLink = async (linkId: string) => {
    try {
      await linkDelete(linkId);
      invalidateEntry();
    } catch (error) {
      showError(error);
    }
  };

  const handleAttach = async () => {
    try {
      const path = await openDialog({ multiple: false, directory: false });
      if (typeof path !== "string") {
        return;
      }
      const attachment = await attachmentAdd(entryId, path);
      const fileName = attachment.localPath.split(/[\\/]/).pop() ?? attachment.fileName;
      const reference =
        attachment.kind === "image"
          ? `![${attachment.fileName}](attachments/${entryId}/${fileName})`
          : `[${attachment.fileName}](attachment:${attachment.id})`;
      const current = draftRef.current;
      if (current) {
        updateDraft({ bodyMd: `${current.bodyMd.trimEnd()}\n\n${reference}\n` });
      }
      invalidateEntry();
      push(t("entries:detail.attachmentAdd"), { tone: "success" });
    } catch (error) {
      showError(error);
    }
  };

  const handleNewSketch = async () => {
    try {
      const sketch = await sketchCreate(entryId);
      const current = draftRef.current;
      if (current) {
        updateDraft({
          bodyMd: `${current.bodyMd.trimEnd()}\n\n<!-- bn:sketch:${sketch.id} -->\n`,
        });
      }
      invalidateEntry();
      setSketchId(sketch.id);
    } catch (error) {
      showError(error);
    }
  };

  const confirmDeleteAttachment = async () => {
    if (!deleteAttachmentId) {
      return;
    }
    try {
      const attachment = entry?.attachments.find((item) => item.id === deleteAttachmentId);
      if (attachment?.kind === "sketch") {
        await sketchDelete(deleteAttachmentId);
      } else {
        await attachmentDelete(deleteAttachmentId);
      }
      setDeleteAttachmentId(null);
      invalidateEntry();
    } catch (error) {
      showError(error);
    }
  };

  const handleDeleteEntry = async () => {
    try {
      await entryDelete(entryId);
      setDeleteOpen(false);
      selectEntry(null);
      void queryClient.invalidateQueries({ queryKey: ["entries"] });
      void queryClient.invalidateQueries({ queryKey: ["stats"] });
      push(t("common:toast.deleted"), {
        tone: "info",
        duration: 10000,
        actionLabel: t("common:actions.restore"),
        onAction: () => {
          void entryRestore(entryId)
            .then(() => {
              void queryClient.invalidateQueries({ queryKey: ["entries"] });
              void queryClient.invalidateQueries({ queryKey: ["stats"] });
            })
            .catch(showError);
        },
      });
    } catch (error) {
      showError(error);
    }
  };

  const handleDuplicate = async () => {
    try {
      const copy = await entryDuplicate(entryId);
      void queryClient.invalidateQueries({ queryKey: ["entries"] });
      navigateEntry(copy.id);
    } catch (error) {
      showError(error);
    }
  };

  const handleExport = async () => {
    try {
      setExporting(true);
      const target = await saveDialog({
        defaultPath: `${draft?.title ?? "note"}.md`,
        filters: [{ name: "Markdown", extensions: ["md"] }],
      });
      if (!target) {
        return;
      }
      const result = await exportEntryMarkdown(entryId, target);
      push(t("entries:detail.export") + ": " + result.path, { tone: "success" });
    } catch (error) {
      showError(error);
    } finally {
      setExporting(false);
    }
  };

  if (entryQuery.isLoading) {
    return (
      <div className="flex flex-1 items-center justify-center text-[var(--text-muted)]">
        <Loader2 className="animate-spin" size={18} />
      </div>
    );
  }

  if (entryQuery.isError || !entry || !schema || !draft) {
    return (
      <div className="flex flex-1 items-center justify-center text-[13px] text-[var(--text-muted)]">
        {t("entries:detail.notFound")}
      </div>
    );
  }

  const trashed = entry.deletedAt !== null;
  const readOnly = trashed || !editing;
  const canEdit = !readOnly;
  const sketches = entry.attachments.filter((attachment) => attachment.kind === "sketch");
  const files = entry.attachments.filter((attachment) => attachment.kind !== "sketch");
  const locale = i18n.language;
  const largeNote = draft.bodyMd.length > 200_000;
  const effectiveMode = largeNote && editorMode === "split" ? "source" : editorMode;

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <div className="flex items-center justify-between gap-2 border-b border-[var(--border)] px-4 py-1.5">
        <div className="flex min-w-0 items-center gap-2">
          <Badge color={schema.color}>{schema.name}</Badge>
          <Select
            className="h-7 w-[180px]"
            value={draft.projectId ?? ""}
            disabled={readOnly}
            onChange={(event) => updateDraft({ projectId: event.target.value || null })}
            aria-label={t("entries:detail.project")}
          >
            <option value="">{t("common:labels.inbox")}</option>
            {projects.map((project) => (
              <option key={project.id} value={project.id}>
                {project.name}
              </option>
            ))}
          </Select>
          {status === "saving" ? (
            <span className="text-[11px] text-[var(--text-muted)]">
              {t("common:states.saving")}
            </span>
          ) : status === "saved" ? (
            <span className="text-[11px] text-[var(--success)]">{t("common:states.saved")}</span>
          ) : status === "error" ? (
            <span className="text-[11px] text-[var(--danger)]">{t("common:states.dirty")}</span>
          ) : null}
        </div>
        <div className="flex items-center gap-0.5">
          {canEdit ? (
            editing ? (
              <Button
                variant="secondary"
                size="sm"
                onClick={() => {
                  if (dirty.current) {
                    void save();
                  }
                  setEditing(false);
                }}
              >
                <Check size={14} />
                {t("common:actions.done")}
              </Button>
            ) : (
              <Button variant="secondary" size="sm" onClick={() => setEditing(true)}>
                <Pencil size={14} />
                {t("common:actions.edit")}
              </Button>
            )
          ) : null}
          <Button
            variant="ghost"
            size="icon"
            title={t("entries:detail.duplicate")}
            aria-label={t("entries:detail.duplicate")}
            onClick={() => void handleDuplicate()}
          >
            <Copy size={15} />
          </Button>
          <Button
            variant="ghost"
            size="icon"
            title={t("entries:detail.export")}
            aria-label={t("entries:detail.export")}
            disabled={exporting}
            onClick={() => void handleExport()}
          >
            <Download size={15} />
          </Button>
          <Button
            variant="ghost"
            size="icon"
            className="text-[var(--danger)]"
            title={t("entries:detail.deleteTitle")}
            aria-label={t("entries:detail.deleteTitle")}
            onClick={() => setDeleteOpen(true)}
          >
            <Trash2 size={15} />
          </Button>
        </div>
      </div>

      {trashed ? (
        <div className="flex items-center justify-between border-b border-[var(--warning)] bg-[var(--warning-soft)] px-4 py-2 text-[12px] text-[var(--warning)]">
          <span>{t("entries:detail.inTrash")}</span>
          <Button
            size="sm"
            onClick={() => {
              void entryRestore(entryId)
                .then(() => invalidateEntry())
                .catch(showError);
            }}
          >
            <RotateCcw size={13} />
            {t("entries:detail.untrash")}
          </Button>
        </div>
      ) : null}

      <div className="min-h-0 flex-1 overflow-auto">
        <div className="mx-auto flex max-w-[1100px] flex-col gap-4 px-5 py-4">
          {readOnly ? (
            <h1 className="w-full text-[26px] font-semibold">
              {draft.title.trim() || t("common:actions.untitled")}
            </h1>
          ) : (
            <input
              value={draft.title}
              aria-label={t("common:labels.notes")}
              onChange={(event) => updateDraft({ title: event.target.value })}
              onBlur={() => void save()}
              className="w-full border-none bg-transparent text-[26px] font-semibold outline-none placeholder:text-[var(--text-muted)]"
              placeholder={t("common:actions.untitled")}
            />
          )}

          <FieldForm
            fields={schema.fields}
            values={draft.values}
            computed={entry.computed}
            disabled={readOnly}
            onChange={setValue}
          />

          <TagsInput
            tags={tags}
            selected={draft.tagIds}
            disabled={readOnly}
            onChange={(tagIds) => updateDraft({ tagIds })}
            onCreate={(name) => createTag.mutate(name)}
          />

          <section className="flex flex-col overflow-hidden rounded-[var(--r-md)] border border-[var(--border)]">
            {readOnly ? null : (
              <header className="flex items-center justify-between border-b border-[var(--border)] bg-[var(--bg-subtle)] px-3 py-1.5">
                <div className="flex gap-1" role="tablist" aria-label={t("editor:mode.split")}>
                  {(["source", "preview", "split"] as const).map((mode) => (
                    <button
                      key={mode}
                      type="button"
                      role="tab"
                      aria-selected={editorMode === mode}
                      onClick={() => setEditorMode(mode)}
                      className={cn(
                        "rounded-[var(--r-sm)] px-2 py-0.5 text-[12px]",
                        editorMode === mode
                          ? "bg-[var(--bg-elevated)] font-medium text-[var(--text)] shadow-[var(--shadow-sm)]"
                          : "text-[var(--text-muted)] hover:text-[var(--text)]",
                      )}
                    >
                      {t(`editor:mode.${mode}`)}
                    </button>
                  ))}
                </div>
                {largeNote ? (
                  <span className="text-[11px] text-[var(--text-muted)]">
                    {t("editor:largeNote")}
                  </span>
                ) : null}
              </header>
            )}
            <div
              className={cn(
                "flex min-h-[360px]",
                !readOnly && effectiveMode === "split" ? "divide-x divide-[var(--border)]" : "",
              )}
            >
              {!readOnly && effectiveMode !== "preview" ? (
                <div className="flex min-h-[360px] min-w-0 flex-1 flex-col">
                  <MarkdownEditor
                    value={draft.bodyMd}
                    onChange={(bodyMd) => updateDraft({ bodyMd })}
                    onSave={() => void save()}
                    readOnly={readOnly}
                    fontSize={settings.editorFontSize}
                    spellCheck={settings.spellCheck}
                  />
                </div>
              ) : null}
              {readOnly || effectiveMode !== "source" ? (
                <div className="flex min-h-[360px] min-w-0 flex-1 flex-col">
                  <MarkdownPreview
                    value={draft.bodyMd}
                    entryId={entry.id}
                    mirrorPath={appInfo.mirrorPath}
                    allowRemoteImages={settings.loadRemoteImages}
                    sketches={sketches}
                    onNavigate={(id) => navigateEntry(id)}
                    onOpenAttachment={(id) => {
                      void attachmentOpen(id).catch(showError);
                    }}
                    onOpenSketch={(id) => setSketchId(id)}
                  />
                </div>
              ) : null}
            </div>
          </section>

          <section>
            <h3 className="mb-2 flex items-center gap-1.5 text-[12px] font-semibold uppercase tracking-wide text-[var(--text-muted)]">
              <ImagePlus size={13} />
              {t("editor:sketch.title")}
            </h3>
            <SketchGallery
              sketches={sketches}
              readOnly={readOnly}
              onNew={() => void handleNewSketch()}
              onOpen={(id) => setSketchId(id)}
              onDelete={(id) => setDeleteAttachmentId(id)}
            />
          </section>

          <section>
            <div className="mb-2 flex items-center justify-between">
              <h3 className="flex items-center gap-1.5 text-[12px] font-semibold uppercase tracking-wide text-[var(--text-muted)]">
                <Paperclip size={13} />
                {t("common:labels.attachments")}
              </h3>
              {!readOnly ? (
                <Button size="sm" variant="ghost" onClick={() => void handleAttach()}>
                  <Plus size={13} />
                  {t("entries:detail.attachmentAdd")}
                </Button>
              ) : null}
            </div>
            {files.length === 0 ? (
              <p className="text-[12px] text-[var(--text-muted)]">
                {t("entries:detail.attachmentEmpty")}
              </p>
            ) : (
              <div className="flex flex-wrap gap-2">
                {files.map((attachment) => (
                  <AttachmentCard
                    key={attachment.id}
                    attachment={attachment}
                    canDelete={!readOnly}
                    onOpen={() => void attachmentOpen(attachment.id).catch(showError)}
                    onDelete={() => setDeleteAttachmentId(attachment.id)}
                  />
                ))}
              </div>
            )}
          </section>

          <LinksSection
            entry={entry}
            candidates={allEntries.data?.items ?? []}
            readOnly={readOnly}
            onAdd={(id) => void handleAddLink(id)}
            onRemove={(id) => void handleRemoveLink(id)}
            onNavigate={(id) => navigateEntry(id)}
          />

          <footer className="flex flex-wrap items-center gap-x-4 gap-y-1 border-t border-[var(--border)] pt-3 text-[11px] text-[var(--text-muted)]">
            <span>
              {t("entries:detail.footer.created", {
                date: formatDate(entry.createdAt, locale, true),
              })}
            </span>
            <span>
              {t("entries:detail.footer.updated", {
                date: formatDate(entry.updatedAt, locale, true),
              })}
            </span>
            <span>{t("entries:detail.footer.words", { count: wordCount(draft.bodyMd) })}</span>
            <span>{t("entries:detail.footer.syncState", { state: entry.syncState })}</span>
            <span className="ml-auto">{formatRelative(entry.updatedAt, locale)}</span>
          </footer>
        </div>
      </div>

      <Dialog
        open={deleteOpen}
        title={t("entries:detail.deleteTitle")}
        onClose={() => setDeleteOpen(false)}
        footer={
          <>
            <Button onClick={() => setDeleteOpen(false)}>{t("common:actions.cancel")}</Button>
            <Button variant="danger" onClick={() => void handleDeleteEntry()}>
              {t("entries:detail.deleteConfirm")}
            </Button>
          </>
        }
      >
        <p className="text-[13px]">{t("entries:detail.deleteMessage", { title: entry.title })}</p>
      </Dialog>

      <Dialog
        open={deleteAttachmentId !== null}
        title={t("entries:detail.attachmentDelete")}
        onClose={() => setDeleteAttachmentId(null)}
        footer={
          <>
            <Button onClick={() => setDeleteAttachmentId(null)}>
              {t("common:actions.cancel")}
            </Button>
            <Button variant="danger" onClick={() => void confirmDeleteAttachment()}>
              {t("common:actions.delete")}
            </Button>
          </>
        }
      >
        <p className="text-[13px]">{t("entries:detail.attachmentDelete")}</p>
      </Dialog>
    </div>
  );
}

function TagsInput({
  tags,
  selected,
  disabled,
  onChange,
  onCreate,
}: {
  tags: TagDto[];
  selected: string[];
  disabled?: boolean;
  onChange: (ids: string[]) => void;
  onCreate: (name: string) => void;
}) {
  const { t } = useTranslation("entries");
  const [value, setValue] = useState("");
  const selectedTags = tags.filter((tag) => selected.includes(tag.id));

  const commit = () => {
    const name = value.trim();
    if (!name) {
      return;
    }
    const existing = tags.find((tag) => tag.name.toLowerCase() === name.toLowerCase());
    if (existing) {
      if (!selected.includes(existing.id)) {
        onChange([...selected, existing.id]);
      }
    } else {
      onCreate(name);
    }
    setValue("");
  };

  return (
    <div className="flex flex-wrap items-center gap-1.5">
      {selectedTags.map((tag) => (
        <span
          key={tag.id}
          className="inline-flex items-center gap-1 rounded-[var(--r-full)] border px-2 py-0.5 text-[11px]"
          style={{
            color: tag.color,
            borderColor: `color-mix(in srgb, ${tag.color} 45%, transparent)`,
            backgroundColor: `color-mix(in srgb, ${tag.color} 12%, transparent)`,
          }}
        >
          {tag.name}
          {!disabled ? (
            <button
              type="button"
              aria-label={`${tag.name} ✕`}
              onClick={() => onChange(selected.filter((id) => id !== tag.id))}
            >
              <X size={11} />
            </button>
          ) : null}
        </span>
      ))}
      {!disabled ? (
        <input
          list="bn-tag-options"
          value={value}
          onChange={(event) => setValue(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === "Enter") {
              event.preventDefault();
              commit();
            }
          }}
          placeholder={t("fields.tagsPlaceholder")}
          className="h-7 w-40 rounded-[var(--r-sm)] border border-transparent bg-transparent px-1 text-[12px] outline-none placeholder:text-[var(--text-muted)] focus:border-[var(--border-strong)]"
        />
      ) : null}
      <datalist id="bn-tag-options">
        {tags.map((tag) => (
          <option key={tag.id} value={tag.name} />
        ))}
      </datalist>
    </div>
  );
}

function AttachmentCard({
  attachment,
  canDelete,
  onOpen,
  onDelete,
}: {
  attachment: AttachmentDto;
  canDelete: boolean;
  onOpen: () => void;
  onDelete: () => void;
}) {
  const { t, i18n } = useTranslation(["entries", "common"]);
  const { state: contextMenu, openAt, close: closeContextMenu } = useContextMenu();
  const isImage = attachment.mimeType.startsWith("image/");
  const entries: ContextMenuEntry[] = [
    {
      id: "open",
      label: t("common:actions.open"),
      icon: <FileText size={13} />,
      onSelect: onOpen,
    },
    {
      id: "copy-path",
      label: t("common:actions.copyPath"),
      icon: <Copy size={13} />,
      onSelect: () => {
        void navigator.clipboard.writeText(attachment.localPath);
      },
    },
  ];
  if (canDelete) {
    entries.push({
      id: "delete",
      label: t("common:actions.delete"),
      icon: <Trash2 size={13} />,
      tone: "danger",
      onSelect: onDelete,
    });
  }
  return (
    <div
      className="group relative flex w-36 flex-col overflow-hidden rounded-[var(--r-md)] border border-[var(--border)]"
      onContextMenu={(event) => openAt(event, entries)}
    >
      <button type="button" onClick={onOpen} className="h-20 overflow-hidden bg-[var(--bg-subtle)]">
        {isImage ? (
          <img
            src={convertFileSrc(attachment.localPath)}
            alt={attachment.fileName}
            className="h-full w-full object-cover"
            loading="lazy"
          />
        ) : (
          <span className="flex h-full items-center justify-center text-[var(--text-muted)]">
            <FileText size={22} />
          </span>
        )}
      </button>
      <div className="flex items-center justify-between gap-1 border-t border-[var(--border)] px-2 py-1">
        <span className="min-w-0">
          <span className="block truncate text-[11px]" title={attachment.fileName}>
            {attachment.fileName}
          </span>
          <span className="block text-[10px] text-[var(--text-muted)]">
            {formatBytes(attachment.byteSize, i18n.language)}
          </span>
        </span>
        <span className="flex gap-0.5">
          <Button
            size="icon"
            variant="ghost"
            className="h-6 w-6"
            title={t("common:actions.open")}
            aria-label={t("common:actions.open")}
            onClick={onOpen}
          >
            <Paperclip size={12} />
          </Button>
          {canDelete ? (
            <Button
              size="icon"
              variant="ghost"
              className="h-6 w-6 text-[var(--danger)]"
              title={t("entries:detail.attachmentDelete")}
              aria-label={t("entries:detail.attachmentDelete")}
              onClick={onDelete}
            >
              <Trash2 size={12} />
            </Button>
          ) : null}
        </span>
      </div>

      {contextMenu ? (
        <ContextMenu
          point={contextMenu.point}
          entries={contextMenu.entries}
          label={attachment.fileName}
          onClose={closeContextMenu}
        />
      ) : null}
    </div>
  );
}

function LinksSection({
  entry,
  candidates,
  readOnly,
  onAdd,
  onRemove,
  onNavigate,
}: {
  entry: EntryDetailDto;
  candidates: EntryListItem[];
  readOnly: boolean;
  onAdd: (id: string) => void;
  onRemove: (id: string) => void;
  onNavigate: (id: string) => void;
}) {
  const { t } = useTranslation("entries");
  const links = entry.links;
  const [target, setTarget] = useState("");
  return (
    <section className="grid grid-cols-1 gap-4 lg:grid-cols-2">
      <div>
        <h3 className="mb-2 flex items-center gap-1.5 text-[12px] font-semibold uppercase tracking-wide text-[var(--text-muted)]">
          <Link2 size={13} />
          {t("detail.addLink")}
        </h3>
        <div className="mb-2 flex gap-1.5">
          {readOnly ? null : (
            <>
              <Select value={target} onChange={(event) => setTarget(event.target.value)}>
                <option value="">{t("detail.linkPlaceholder")}</option>
                {candidates
                  .filter(
                    (candidate) =>
                      candidate.id !== entry.id &&
                      !links.links.some((link) => link.entryId === candidate.id),
                  )
                  .map((candidate) => (
                    <option key={candidate.id} value={candidate.id}>
                      {candidate.title}
                    </option>
                  ))}
              </Select>
              <Button
                onClick={() => {
                  if (target) {
                    onAdd(target);
                    setTarget("");
                  }
                }}
              >
                <Plus size={13} />
              </Button>
            </>
          )}
        </div>
        {links.links.length === 0 ? (
          <p className="text-[12px] text-[var(--text-muted)]">{t("detail.noLinks")}</p>
        ) : (
          <ul className="space-y-1">
            {links.links.map((link) => (
              <li key={link.entryId} className="flex items-center gap-1.5 text-[12px]">
                <ArrowRightLeft size={12} className="text-[var(--text-muted)]" />
                <button
                  type="button"
                  className="truncate hover:underline"
                  onClick={() => onNavigate(link.entryId)}
                >
                  {link.title}
                </button>
                {!readOnly && link.id ? (
                  <button
                    type="button"
                    className="ml-auto text-[var(--text-muted)] hover:text-[var(--danger)]"
                    aria-label={t("detail.linkRemove")}
                    onClick={() => onRemove(link.id as string)}
                  >
                    <X size={12} />
                  </button>
                ) : null}
              </li>
            ))}
          </ul>
        )}
        {links.markdownLinks.length > 0 ? (
          <div className="mt-2">
            <p className="text-[11px] uppercase tracking-wide text-[var(--text-muted)]">
              {t("detail.markdownLinks")}
            </p>
            <ul className="space-y-0.5">
              {links.markdownLinks.map((link) => (
                <li key={link.entryId}>
                  <button
                    type="button"
                    className="truncate text-[12px] text-[var(--accent)] hover:underline"
                    onClick={() => onNavigate(link.entryId)}
                  >
                    {link.title}
                  </button>
                </li>
              ))}
            </ul>
          </div>
        ) : null}
      </div>
      <div>
        <h3 className="mb-2 text-[12px] font-semibold uppercase tracking-wide text-[var(--text-muted)]">
          {t("common:labels.backlinks")}
        </h3>
        {links.backlinks.length === 0 && links.markdownBacklinks.length === 0 ? (
          <p className="text-[12px] text-[var(--text-muted)]">{t("detail.noBacklinks")}</p>
        ) : (
          <ul className="space-y-1">
            {[...links.backlinks, ...links.markdownBacklinks].map((link, index) => (
              <li key={`${link.entryId}-${index}`} className="text-[12px]">
                <button
                  type="button"
                  className="truncate hover:underline"
                  onClick={() => onNavigate(link.entryId)}
                >
                  {link.title}
                </button>
              </li>
            ))}
          </ul>
        )}
        {links.markdownBacklinks.length > 0 ? (
          <p className="mt-2 text-[11px] text-[var(--text-muted)]">
            {t("detail.markdownBacklinks")}
          </p>
        ) : null}
      </div>
    </section>
  );
}
