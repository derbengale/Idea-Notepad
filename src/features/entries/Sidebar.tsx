import { useUi } from "@/app/store";
import { Button } from "@/components/ui/button";
import { ContextMenu, useContextMenu, type ContextMenuEntry } from "@/components/ui/context-menu";
import { Select } from "@/components/ui/select";
import { SyncIndicator } from "@/features/sync/SyncIndicator";
import type { EntryListItem, ProjectDto, SchemaDto, TagDto } from "@/lib/ipc/types";
import { cn } from "@/lib/utils/cn";
import { FileText, Folder, Inbox, Layers, Pencil, Plus, Settings, Tag } from "lucide-react";
import { useMemo, useState, type ReactNode } from "react";
import { useTranslation } from "react-i18next";

export interface SidebarProps {
  schemas: SchemaDto[];
  projects: ProjectDto[];
  tags: TagDto[];
  entries: EntryListItem[];
  version: string;
  defaultSchemaId: string | null;
  onNewNote: (schemaId: string) => void;
  onNewProject: () => void;
}

export function Sidebar({
  schemas,
  projects,
  tags,
  entries,
  version,
  defaultSchemaId,
  onNewNote,
  onNewProject,
}: SidebarProps) {
  const { t } = useTranslation(["common", "entries", "projects"]);
  const { state: contextMenu, openAt, close: closeContextMenu } = useContextMenu();
  const {
    schemaFilter,
    projectFilter,
    tagFilters,
    trash,
    setSchemaFilter,
    setProjectFilter,
    toggleTag,
    setTrash,
    openSettings,
    openSchemaEditor,
  } = useUi();
  const [selectedSchema, setSelectedSchema] = useState(defaultSchemaId ?? schemas[0]?.id ?? "");

  const counts = useMemo(() => {
    const bySchema = new Map<string, number>();
    const byProject = new Map<string, number>();
    const byTag = new Map<string, number>();
    for (const entry of entries) {
      bySchema.set(entry.schemaId, (bySchema.get(entry.schemaId) ?? 0) + 1);
      if (entry.projectId) {
        byProject.set(entry.projectId, (byProject.get(entry.projectId) ?? 0) + 1);
      }
      for (const tag of entry.tags) {
        byTag.set(tag.id, (byTag.get(tag.id) ?? 0) + 1);
      }
    }
    return { bySchema, byProject, byTag };
  }, [entries]);

  const activeSchema = schemas.find((schema) => schema.id === selectedSchema) ?? schemas[0];

  const buildSchemaMenu = (schema: SchemaDto): ContextMenuEntry[] => [
    {
      id: "new-note",
      label: t("entries:newWithSchema"),
      icon: <Plus size={13} />,
      onSelect: () => onNewNote(schema.id),
    },
    {
      id: "filter",
      label: t("entries:filterBy"),
      icon: <FileText size={13} />,
      checked: schemaFilter === schema.id,
      onSelect: () => {
        setSchemaFilter(schema.id);
        setTrash(false);
      },
    },
    { id: "separator-schema", separator: true },
    {
      id: "edit-schema",
      label: t("entries:editSchema"),
      icon: <Pencil size={13} />,
      onSelect: () => openSchemaEditor(schema.id),
    },
  ];

  return (
    <nav
      aria-label={t("common:labels.schemas")}
      className="flex min-h-0 flex-1 flex-col bg-[var(--bg-elevated)]"
    >
      <div className="space-y-2 border-b border-[var(--border)] px-3 py-3">
        <Button
          variant="primary"
          className="w-full"
          disabled={!activeSchema}
          onClick={() => activeSchema && onNewNote(activeSchema.id)}
        >
          <Plus size={15} />
          {t("common:actions.newNote")}
        </Button>
        <Select
          aria-label={t("common:actions.newNote")}
          value={selectedSchema}
          onChange={(event) => setSelectedSchema(event.target.value)}
          className="h-7 text-[12px]"
        >
          {schemas.map((schema) => (
            <option key={schema.id} value={schema.id}>
              {schema.name}
            </option>
          ))}
        </Select>
      </div>

      <div className="min-h-0 flex-1 overflow-auto px-2 py-2">
        <NavItem
          active={!schemaFilter && !projectFilter && tagFilters.length === 0 && !trash}
          onClick={() => {
            setSchemaFilter(null);
            setProjectFilter(null);
            setTrash(false);
          }}
          icon={<Layers size={14} />}
          label={t("common:labels.all")}
          count={entries.length}
        />

        <SectionHeader
          title={t("common:labels.schemas")}
          action={
            <button
              type="button"
              className="text-[var(--text-muted)] hover:text-[var(--text)]"
              title={t("common:actions.newSchema")}
              aria-label={t("common:actions.newSchema")}
              onClick={() => openSchemaEditor(null)}
            >
              <Plus size={13} />
            </button>
          }
        />
        {schemas.map((schema, index) => (
          <NavItem
            key={schema.id}
            active={schemaFilter === schema.id}
            onClick={() => {
              setSchemaFilter(schema.id);
              setTrash(false);
            }}
            icon={<FileText size={14} style={{ color: schema.color }} />}
            label={schema.name}
            count={counts.bySchema.get(schema.id) ?? 0}
            suffix={index < 9 ? <kbd className="text-[10px] opacity-50">⌃{index + 1}</kbd> : null}
            onDoubleClick={() => openSchemaEditor(schema.id)}
            onContextMenu={(event) => openAt(event, buildSchemaMenu(schema))}
            title={t("common:actions.edit")}
          />
        ))}

        <SectionHeader
          title={t("common:labels.projects")}
          action={
            <button
              type="button"
              className="text-[var(--text-muted)] hover:text-[var(--text)]"
              title={t("projects:new")}
              aria-label={t("projects:new")}
              onClick={onNewProject}
            >
              <Plus size={13} />
            </button>
          }
        />
        <NavItem
          active={projectFilter === "inbox"}
          onClick={() => {
            setProjectFilter("inbox");
            setTrash(false);
          }}
          icon={<Inbox size={14} />}
          label={t("common:labels.inbox")}
        />
        {projects.map((project) => (
          <NavItem
            key={project.id}
            active={projectFilter === project.id}
            onClick={() => {
              setProjectFilter(project.id);
              setTrash(false);
            }}
            icon={<Folder size={14} style={{ color: project.color }} />}
            label={project.name}
            count={counts.byProject.get(project.id) ?? project.noteCount}
          />
        ))}

        {tags.length > 0 ? (
          <>
            <SectionHeader title={t("common:labels.tags")} />
            <div className="flex flex-wrap gap-1 px-2 py-1">
              {tags.slice(0, 10).map((tag) => (
                <button
                  key={tag.id}
                  type="button"
                  onClick={() => toggleTag(tag.id)}
                  className={cn(
                    "inline-flex items-center gap-1 rounded-[var(--r-full)] border px-2 py-0.5 text-[11px]",
                    tagFilters.includes(tag.id)
                      ? "border-[var(--accent)] bg-[var(--accent-soft)] text-[var(--accent)]"
                      : "border-[var(--border)] text-[var(--text-muted)] hover:bg-[var(--bg-subtle)]",
                  )}
                >
                  <span
                    className="h-1.5 w-1.5 rounded-full"
                    style={{ backgroundColor: tag.color }}
                  />
                  {tag.name}
                </button>
              ))}
            </div>
          </>
        ) : null}

        <div className="mt-2 border-t border-[var(--border)] pt-2">
          <NavItem
            active={trash}
            onClick={() => setTrash(!trash)}
            icon={<Tag size={14} />}
            label={t("common:labels.trash")}
          />
        </div>
      </div>

      <div className="border-t border-[var(--border)] px-3 py-2">
        <SyncIndicator />
        <div className="mt-1 flex items-center justify-between">
          <Button variant="ghost" size="sm" onClick={() => openSettings("general")}>
            <Settings size={13} />
            {t("common:actions.settings")}
          </Button>
          <span className="text-[10px] text-[var(--text-muted)]">v{version}</span>
        </div>
      </div>

      {contextMenu ? (
        <ContextMenu
          point={contextMenu.point}
          entries={contextMenu.entries}
          label={t("common:labels.actions")}
          onClose={closeContextMenu}
        />
      ) : null}
    </nav>
  );
}

function SectionHeader({ title, action }: { title: string; action?: ReactNode }) {
  return (
    <div className="mt-3 flex items-center justify-between px-2 py-1">
      <span className="text-[10px] font-semibold uppercase tracking-wider text-[var(--text-muted)]">
        {title}
      </span>
      {action}
    </div>
  );
}

function NavItem({
  active,
  onClick,
  onDoubleClick,
  onContextMenu,
  icon,
  label,
  count,
  suffix,
  title,
}: {
  active: boolean;
  onClick: () => void;
  onDoubleClick?: () => void;
  onContextMenu?: (event: React.MouseEvent<HTMLButtonElement>) => void;
  icon: React.ReactNode;
  label: string;
  count?: number;
  suffix?: ReactNode;
  title?: string;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      onDoubleClick={onDoubleClick}
      onContextMenu={onContextMenu}
      title={title ?? label}
      className={cn(
        "flex w-full items-center gap-2 rounded-[var(--r-sm)] px-2 py-1.5 text-left text-[12.5px]",
        active
          ? "bg-[var(--accent-soft)] font-medium text-[var(--accent)]"
          : "text-[var(--text)] hover:bg-[var(--bg-subtle)]",
      )}
    >
      <span className="shrink-0 text-[var(--text-muted)]">{icon}</span>
      <span className="min-w-0 flex-1 truncate">{label}</span>
      {suffix}
      {typeof count === "number" && count > 0 ? (
        <span className="text-[10.5px] text-[var(--text-muted)]">{count}</span>
      ) : null}
    </button>
  );
}
