import { useUi } from "@/app/store";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { ContextMenu, useContextMenu, type ContextMenuEntry } from "@/components/ui/context-menu";
import { Select } from "@/components/ui/select";
import type { EntryListItem, ProjectDto, SchemaDto, TagDto } from "@/lib/ipc/types";
import { cn } from "@/lib/utils/cn";
import { formatRelative } from "@/lib/utils/format";
import {
    ArrowUpDown,
    Copy,
    Download,
    FileText,
    Folder,
    FolderInput,
    Inbox,
    Link2,
    Pencil,
    RotateCcw,
    Trash2,
    X,
} from "lucide-react";
import { useMemo } from "react";
import { useTranslation } from "react-i18next";

export interface EntryListProps {
  items: EntryListItem[];
  total: number;
  loading: boolean;
  schemas: SchemaDto[];
  projects: ProjectDto[];
  tags: TagDto[];
  onSelect: (id: string) => void;
  onEdit: (id: string) => void;
  onRestore: (id: string) => void;
  onNewNote: () => void;
  onDuplicate: (id: string) => void;
  onDelete: (id: string) => void;
  onMoveProject: (id: string, projectId: string | null) => void;
  onLink: (id: string) => void;
  onExport: (id: string) => void;
}

export function EntryList({
  items,
  total,
  loading,
  schemas,
  projects,
  tags,
  onSelect,
  onEdit,
  onRestore,
  onNewNote,
  onDuplicate,
  onDelete,
  onMoveProject,
  onLink,
  onExport,
}: EntryListProps) {
  const { t, i18n } = useTranslation(["entries", "common"]);
  const {
    selectedEntryId,
    schemaFilter,
    projectFilter,
    tagFilters,
    search,
    trash,
    sort,
    density,
    setSort,
    setDensity,
    setSchemaFilter,
    setProjectFilter,
    toggleTag,
    setTrash,
    setSearch,
    clearFilters,
  } = useUi();

  const { state: contextMenu, openAt, close: closeContextMenu } = useContextMenu();

  const activeSchema = schemas.find((schema) => schema.id === schemaFilter);
  const activeProject = projects.find((project) => project.id === projectFilter);
  const activeTags = tags.filter((tag) => tagFilters.includes(tag.id));
  const hasFilters =
    Boolean(schemaFilter) || Boolean(projectFilter) || tagFilters.length > 0 || Boolean(search);
  const sortFields = useMemo(() => {
    const candidates = activeSchema
      ? activeSchema.fields
      : schemas.flatMap((schema) => schema.fields);
    const seen = new Set<string>();
    return candidates.filter((field) => {
      if (!field.showInList || field.type === "tags" || field.type === "longtext") {
        return false;
      }
      if (seen.has(field.key)) {
        return false;
      }
      seen.add(field.key);
      return true;
    });
  }, [activeSchema, schemas]);

  const buildEntryMenu = (item: EntryListItem): ContextMenuEntry[] => {
    const entries: ContextMenuEntry[] = [
      {
        id: "open",
        label: t("common:actions.open"),
        icon: <FileText size={13} />,
        onSelect: () => onSelect(item.id),
      },
    ];
    if (!item.deletedAt) {
      entries.push(
        {
          id: "edit",
          label: t("common:actions.edit"),
          icon: <Pencil size={13} />,
          onSelect: () => onEdit(item.id),
        },
        {
          id: "link",
          label: t("entries:detail.addLink"),
          icon: <Link2 size={13} />,
          onSelect: () => onLink(item.id),
        },
      );
    }
    entries.push({
      id: "duplicate",
      label: t("entries:detail.duplicate"),
      icon: <Copy size={13} />,
      onSelect: () => onDuplicate(item.id),
    });
    if (!item.deletedAt) {
      entries.push({
        id: "move",
        label: t("entries:detail.moveToProject"),
        icon: <FolderInput size={13} />,
        submenu: [
          {
            id: "move-inbox",
            label: t("common:labels.inbox"),
            icon: <Inbox size={13} />,
            checked: item.projectId === null,
            onSelect: () => onMoveProject(item.id, null),
          },
          ...projects.map((project) => ({
            id: `move-${project.id}`,
            label: project.name,
            icon: <Folder size={13} />,
            checked: item.projectId === project.id,
            onSelect: () => onMoveProject(item.id, project.id),
          })),
        ],
      });
    }
    entries.push({
      id: "export",
      label: t("entries:detail.export"),
      icon: <Download size={13} />,
      onSelect: () => onExport(item.id),
    });
    entries.push({ id: "separator-actions", separator: true });
    if (item.deletedAt) {
      entries.push({
        id: "restore",
        label: t("entries:trash.restore"),
        icon: <RotateCcw size={13} />,
        onSelect: () => onRestore(item.id),
      });
    } else {
      entries.push({
        id: "delete",
        label: t("common:actions.delete"),
        icon: <Trash2 size={13} />,
        tone: "danger",
        onSelect: () => onDelete(item.id),
      });
    }
    return entries;
  };

  return (
    <main className="flex min-h-0 flex-1 flex-col" aria-label={t("common:labels.notes")}>
      <div className="flex flex-wrap items-center gap-1.5 border-b border-[var(--border)] px-3 py-2">
        {activeSchema ? (
          <Chip label={activeSchema.name} onRemove={() => setSchemaFilter(null)} />
        ) : null}
        {activeProject ? (
          <Chip label={activeProject.name} onRemove={() => setProjectFilter(null)} />
        ) : null}
        {activeTags.map((tag) => (
          <Chip
            key={tag.id}
            label={tag.name}
            color={tag.color}
            onRemove={() => toggleTag(tag.id)}
          />
        ))}
        {trash ? <Chip label={t("common:labels.trash")} onRemove={() => setTrash(false)} /> : null}
        {hasFilters ? (
          <Button
            variant="ghost"
            size="sm"
            className="h-6 px-1.5 text-[11px]"
            onClick={clearFilters}
          >
            {t("entries:clearFilters")}
          </Button>
        ) : null}
        <span className="ml-auto text-[11px] text-[var(--text-muted)]">
          {t("common:labels.results", { count: items.length, total })}
        </span>
      </div>

      <div className="flex items-center gap-1.5 border-b border-[var(--border)] px-3 py-1.5">
        <ArrowUpDown size={12} className="text-[var(--text-muted)]" />
        <Select
          className="h-6 w-auto min-w-[120px] py-0 text-[11px]"
          value={sort.key}
          onChange={(event) => setSort({ key: event.target.value, direction: sort.direction })}
          aria-label={t("common:labels.sort")}
        >
          <option value="updated">{t("entries:sort.updated")}</option>
          <option value="created">{t("entries:sort.created")}</option>
          <option value="title">{t("entries:sort.title")}</option>
          {sortFields.map((field) => (
            <option key={field.id} value={field.key}>
              {field.label}
            </option>
          ))}
        </Select>
        <Button
          size="sm"
          variant="ghost"
          className="h-6 px-1.5 text-[11px]"
          onClick={() =>
            setSort({ key: sort.key, direction: sort.direction === "asc" ? "desc" : "asc" })
          }
          title={sort.direction === "asc" ? "A–Z" : "Z–A"}
        >
          {sort.direction === "asc" ? "↑" : "↓"}
        </Button>
        <Button
          size="sm"
          variant="ghost"
          className="h-6 px-1.5 text-[11px]"
          onClick={() => setDensity(density === "comfortable" ? "compact" : "comfortable")}
          title={t("common:labels.density")}
        >
          {density === "comfortable"
            ? t("entries:density.comfortable")
            : t("entries:density.compact")}
        </Button>
        <Button
          size="sm"
          variant={search ? "secondary" : "ghost"}
          className={cn("h-6 px-1.5 text-[11px]", search ? "max-w-[160px] truncate" : "")}
          onClick={() => setSearch("")}
          title={search ? `${t("common:actions.search")}: ${search}` : t("common:actions.search")}
        >
          {search ? `“${search}”` : t("common:actions.search")}
        </Button>
      </div>

      <div className="min-h-0 flex-1 overflow-auto">
        {loading ? (
          <div className="space-y-2 p-3">
            {Array.from({ length: 6 }).map((_, index) => (
              <div
                key={index}
                className="h-12 animate-pulse rounded-[var(--r-sm)] bg-[var(--bg-subtle)]"
              />
            ))}
          </div>
        ) : items.length === 0 ? (
          <div className="flex h-full flex-col items-center justify-center gap-2 px-6 text-center">
            {trash ? (
              <>
                <Trash2 size={22} className="text-[var(--text-muted)]" />
                <p className="text-[13px] text-[var(--text-muted)]">{t("entries:trash.empty")}</p>
              </>
            ) : hasFilters ? (
              <>
                <p className="text-[13px] text-[var(--text-muted)]">{t("entries:noResults")}</p>
                <Button size="sm" onClick={clearFilters}>
                  {t("entries:clearFilters")}
                </Button>
              </>
            ) : (
              <>
                <p className="text-[14px] font-medium">{t("entries:empty.title")}</p>
                <p className="max-w-[280px] text-[12px] text-[var(--text-muted)]">
                  {t("entries:empty.description")}
                </p>
                <Button size="sm" variant="primary" onClick={onNewNote}>
                  {t("entries:empty.action")}
                </Button>
              </>
            )}
          </div>
        ) : (
          <ul className="divide-y divide-[var(--border)]">
            {items.map((item) => {
              const schema = schemas.find((candidate) => candidate.id === item.schemaId);
              const selected = selectedEntryId === item.id;
              return (
                <li
                  key={item.id}
                  className="group relative"
                  onContextMenu={(event) => openAt(event, buildEntryMenu(item))}
                >
                  <button
                    type="button"
                    onClick={() => onSelect(item.id)}
                    className={cn(
                      "flex w-full flex-col gap-1 border-l-2 px-3 text-left",
                      density === "compact" ? "py-1.5" : "py-2.5",
                      selected
                        ? "border-[var(--accent)] bg-[var(--accent-soft)]"
                        : "border-transparent hover:bg-[var(--bg-subtle)]",
                    )}
                  >
                    <span className="flex items-center gap-2">
                      <span
                        className="h-2 w-2 shrink-0 rounded-full"
                        style={{ backgroundColor: schema?.color ?? "var(--text-muted)" }}
                      />
                      <span className="min-w-0 flex-1 truncate text-[13px] font-medium">
                        {item.title}
                      </span>
                      {item.badge ? <Badge color={item.badgeColor}>{item.badge}</Badge> : null}
                    </span>
                    <span className="flex items-center gap-1.5 pl-4 text-[11px] text-[var(--text-muted)]">
                      {item.subtitle ? (
                        <span className="min-w-0 truncate">{item.subtitle}</span>
                      ) : (
                        <span>{schema?.name ?? item.schemaKey}</span>
                      )}
                      <span>·</span>
                      <span className="shrink-0">
                        {formatRelative(item.updatedAt, i18n.language)}
                      </span>
                      {item.deletedAt ? (
                        <span className="ml-auto flex items-center gap-1">
                          <span
                            role="button"
                            tabIndex={0}
                            className="rounded px-1 text-[var(--accent)] hover:underline"
                            onClick={(event) => {
                              event.stopPropagation();
                              onRestore(item.id);
                            }}
                            onKeyDown={(event) => {
                              if (event.key === "Enter") {
                                event.stopPropagation();
                                onRestore(item.id);
                              }
                            }}
                          >
                            <RotateCcw size={11} className="mr-0.5 inline" />
                            {t("entries:trash.restore")}
                          </span>
                        </span>
                      ) : null}
                    </span>
                    {item.tags.length > 0 ? (
                      <span className="flex flex-wrap gap-1 pl-4">
                        {item.tags.slice(0, 4).map((tag) => (
                          <span
                            key={tag.id}
                            className="rounded-[var(--r-full)] px-1.5 py-0.5 text-[10px]"
                            style={{
                              color: tag.color,
                              backgroundColor: `color-mix(in srgb, ${tag.color} 14%, transparent)`,
                            }}
                          >
                            #{tag.name}
                          </span>
                        ))}
                      </span>
                    ) : null}
                  </button>
                  {item.deletedAt ? null : (
                    <button
                      type="button"
                      onClick={() => onEdit(item.id)}
                      title={t("common:actions.edit")}
                      aria-label={t("common:actions.edit")}
                      className="absolute right-2 top-2 rounded-[var(--r-sm)] p-1 text-[var(--text-muted)] opacity-0 transition-opacity hover:bg-[var(--bg-elevated)] hover:text-[var(--text)] focus-visible:opacity-100 group-hover:opacity-100"
                    >
                      <Pencil size={12} />
                    </button>
                  )}
                </li>
              );
            })}
          </ul>
        )}
      </div>

      {contextMenu ? (
        <ContextMenu
          point={contextMenu.point}
          entries={contextMenu.entries}
          label={t("common:labels.actions")}
          onClose={closeContextMenu}
        />
      ) : null}
    </main>
  );
}

function Chip({ label, color, onRemove }: { label: string; color?: string; onRemove: () => void }) {
  return (
    <span
      className="inline-flex items-center gap-1 rounded-[var(--r-full)] border border-[var(--border-strong)] px-2 py-0.5 text-[11px]"
      style={color ? { color, borderColor: color } : undefined}
    >
      {label}
      <button type="button" onClick={onRemove} aria-label={`${label} ✕`}>
        <X size={10} />
      </button>
    </span>
  );
}
