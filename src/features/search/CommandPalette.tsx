import { useIpcError } from "@/app/errors";
import { useUi } from "@/app/store";
import { useToast } from "@/components/ui/toast";
import {
  entryCreate,
  entryList,
  rebuildIndex,
  schemaList,
  settingsGet,
  settingsSet,
  syncNow,
} from "@/lib/ipc";
import { cn } from "@/lib/utils/cn";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import {
  ArrowRight,
  FilePlus2,
  FolderPlus,
  Languages,
  Moon,
  RefreshCw,
  Settings,
  Sun,
  Trash2,
} from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { useTranslation } from "react-i18next";

interface Command {
  id: string;
  group: string;
  label: string;
  icon: ReactNode;
  run: () => void;
}

export function CommandPalette() {
  const { t } = useTranslation(["common", "entries"]);
  const queryClient = useQueryClient();
  const showError = useIpcError();
  const { push } = useToast();
  const {
    paletteOpen,
    setPaletteOpen,
    navigateEntry,
    openSchemaEditor,
    openSettings,
    setTrash,
    setSchemaFilter,
    setShareMode,
    setProjectDialogOpen,
    history,
  } = useUi();
  const [query, setQuery] = useState("");
  const [active, setActive] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);

  const schemas = useQuery({ queryKey: ["schemas"], queryFn: schemaList });
  const entries = useQuery({
    queryKey: ["entries", "palette"],
    queryFn: () => entryList({ limit: 500 }),
    enabled: paletteOpen,
  });
  const settings = useQuery({ queryKey: ["settings"], queryFn: settingsGet });

  useEffect(() => {
    if (paletteOpen) {
      setQuery("");
      setActive(0);
      window.setTimeout(() => inputRef.current?.focus(), 20);
    }
  }, [paletteOpen]);

  const createNote = useCallback(
    async (schemaId: string) => {
      try {
        const entry = await entryCreate({
          schemaId,
          projectId: null,
          title: t("common:actions.untitled"),
          bodyMd: "",
          values: {},
          tagIds: [],
          links: [],
        });
        void queryClient.invalidateQueries({ queryKey: ["entries"] });
        navigateEntry(entry.id);
        setPaletteOpen(false);
      } catch (error) {
        showError(error);
      }
    },
    [navigateEntry, queryClient, setPaletteOpen, showError, t],
  );

  const commands = useMemo<Command[]>(() => {
    const list: Command[] = [];
    for (const schema of schemas.data ?? []) {
      list.push({
        id: `new-${schema.id}`,
        group: t("common:palette.actions"),
        label: `${t("common:actions.newNote")} · ${schema.name}`,
        icon: <FilePlus2 size={14} />,
        run: () => void createNote(schema.id),
      });
    }
    list.push(
      {
        id: "new-schema",
        group: t("common:palette.actions"),
        label: t("common:actions.newSchema"),
        icon: <FilePlus2 size={14} />,
        run: () => {
          setPaletteOpen(false);
          openSchemaEditor(null);
        },
      },
      {
        id: "new-project",
        group: t("common:palette.actions"),
        label: t("common:actions.newProject"),
        icon: <FolderPlus size={14} />,
        run: () => {
          setPaletteOpen(false);
          setProjectDialogOpen(true);
        },
      },
      {
        id: "sync",
        group: t("common:palette.actions"),
        label: t("common:actions.syncNow"),
        icon: <RefreshCw size={14} />,
        run: () => {
          setPaletteOpen(false);
          void syncNow()
            .then(() => queryClient.invalidateQueries())
            .catch(showError);
        },
      },
      {
        id: "rebuild",
        group: t("common:palette.actions"),
        label: t("common:labels.diagnostics"),
        icon: <RefreshCw size={14} />,
        run: () => {
          setPaletteOpen(false);
          void rebuildIndex()
            .then((report) =>
              push(t("common:toast.saved") + ` (${report.notes})`, { tone: "success" }),
            )
            .catch(showError);
        },
      },
      {
        id: "theme",
        group: t("common:palette.actions"),
        label: t("common:labels.theme"),
        icon:
          document.documentElement.dataset.theme === "dark" ? (
            <Sun size={14} />
          ) : (
            <Moon size={14} />
          ),
        run: () => {
          const current = settings.data;
          if (!current) {
            return;
          }
          const next = current.theme === "dark" ? "light" : "dark";
          void settingsSet({ ...current, theme: next })
            .then((value) => queryClient.setQueryData(["settings"], value))
            .catch(showError);
          setPaletteOpen(false);
        },
      },
      {
        id: "language",
        group: t("common:palette.actions"),
        label: t("common:labels.language"),
        icon: <Languages size={14} />,
        run: () => {
          const current = settings.data;
          if (!current) {
            return;
          }
          const next = current.language === "de" ? "en" : "de";
          void settingsSet({ ...current, language: next })
            .then((value) => queryClient.setQueryData(["settings"], value))
            .catch(showError);
          setPaletteOpen(false);
        },
      },
      {
        id: "settings",
        group: t("common:palette.actions"),
        label: t("common:actions.settings"),
        icon: <Settings size={14} />,
        run: () => {
          setPaletteOpen(false);
          openSettings("general");
        },
      },
      {
        id: "export",
        group: t("common:palette.actions"),
        label: t("common:actions.export"),
        icon: <ArrowRight size={14} />,
        run: () => {
          setPaletteOpen(false);
          setShareMode("export");
        },
      },
      {
        id: "import",
        group: t("common:palette.actions"),
        label: t("common:actions.import"),
        icon: <ArrowRight size={14} />,
        run: () => {
          setPaletteOpen(false);
          setShareMode("import");
        },
      },
      {
        id: "trash",
        group: t("common:palette.navigation"),
        label: t("common:labels.trash"),
        icon: <Trash2 size={14} />,
        run: () => {
          setPaletteOpen(false);
          setTrash(true);
        },
      },
    );
    for (const schema of schemas.data ?? []) {
      list.push({
        id: `schema-${schema.id}`,
        group: t("common:palette.navigation"),
        label: schema.name,
        icon: <span style={{ color: schema.color }}>●</span>,
        run: () => {
          setPaletteOpen(false);
          setSchemaFilter(schema.id);
        },
      });
    }
    return list;
  }, [
    schemas.data,
    settings.data,
    t,
    openSchemaEditor,
    openSettings,
    setPaletteOpen,
    setProjectDialogOpen,
    setSchemaFilter,
    setShareMode,
    setTrash,
    push,
    queryClient,
    showError,
    createNote,
  ]);

  const noteCommands = useMemo<Command[]>(() => {
    const term = query.trim().toLowerCase();
    return (entries.data?.items ?? [])
      .filter((entry) => !term || entry.title.toLowerCase().includes(term))
      .slice(0, 8)
      .map((entry) => ({
        id: `note-${entry.id}`,
        group: t("common:palette.notes"),
        label: entry.title,
        icon: <span style={{ color: "var(--text-muted)" }}>{entry.schemaKey}</span>,
        run: () => {
          navigateEntry(entry.id);
          setPaletteOpen(false);
        },
      }));
  }, [entries.data, query, navigateEntry, setPaletteOpen, t]);

  const filteredCommands = useMemo(() => {
    const term = query.trim().toLowerCase();
    const filtered = term
      ? commands.filter((command) => command.label.toLowerCase().includes(term))
      : commands;
    return filtered.slice(0, 12);
  }, [commands, query]);

  const items = useMemo(
    () => [...noteCommands, ...filteredCommands],
    [noteCommands, filteredCommands],
  );
  const grouped = useMemo(() => {
    const map = new Map<string, Command[]>();
    for (const item of items) {
      const list = map.get(item.group) ?? [];
      list.push(item);
      map.set(item.group, list);
    }
    return [...map.entries()];
  }, [items]);

  useEffect(() => {
    setActive(0);
  }, [query, paletteOpen]);

  if (!paletteOpen) {
    return null;
  }

  const run = (command: Command | undefined) => {
    if (command) {
      command.run();
    }
  };

  return (
    <div
      className="fixed inset-0 z-50 flex items-start justify-center bg-black/40 pt-[12vh]"
      onMouseDown={(event) => {
        if (event.target === event.currentTarget) {
          setPaletteOpen(false);
        }
      }}
    >
      <div className="w-[560px] overflow-hidden rounded-[var(--r-lg)] border border-[var(--border)] bg-[var(--bg-elevated)] shadow-[var(--shadow-md)]">
        <input
          ref={inputRef}
          value={query}
          onChange={(event) => setQuery(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === "Escape") {
              setPaletteOpen(false);
            } else if (event.key === "ArrowDown") {
              event.preventDefault();
              setActive((current) => Math.min(items.length - 1, current + 1));
            } else if (event.key === "ArrowUp") {
              event.preventDefault();
              setActive((current) => Math.max(0, current - 1));
            } else if (event.key === "Enter") {
              event.preventDefault();
              run(items[active]);
            }
          }}
          placeholder={t("common:palette.placeholder")}
          className="w-full border-b border-[var(--border)] bg-transparent px-4 py-3 text-[14px] outline-none placeholder:text-[var(--text-muted)]"
        />
        <div className="max-h-[50vh] overflow-auto p-1.5">
          {items.length === 0 ? (
            <p className="px-3 py-4 text-[12.5px] text-[var(--text-muted)]">
              {t("common:palette.noResults")}
            </p>
          ) : (
            grouped.map(([group, commandsInGroup]) => (
              <div key={group}>
                <p className="px-2 pb-1 pt-2 text-[10px] font-semibold uppercase tracking-wider text-[var(--text-muted)]">
                  {group}
                </p>
                {commandsInGroup.map((command) => {
                  const index = items.indexOf(command);
                  return (
                    <button
                      key={command.id}
                      type="button"
                      onMouseEnter={() => setActive(index)}
                      onClick={() => run(command)}
                      className={cn(
                        "flex w-full items-center gap-2 rounded-[var(--r-sm)] px-2 py-1.5 text-left text-[12.5px]",
                        index === active
                          ? "bg-[var(--accent-soft)] text-[var(--accent)]"
                          : "hover:bg-[var(--bg-subtle)]",
                      )}
                    >
                      <span className="text-[var(--text-muted)]">{command.icon}</span>
                      <span className="min-w-0 flex-1 truncate">{command.label}</span>
                    </button>
                  );
                })}
              </div>
            ))
          )}
        </div>
        <div className="flex items-center justify-between border-t border-[var(--border)] px-3 py-1.5 text-[10.5px] text-[var(--text-muted)]">
          <span>{history.length > 0 ? t("common:actions.back") : ""}</span>
          <span>↑ ↓ · Enter · Esc</span>
        </div>
      </div>
    </div>
  );
}
