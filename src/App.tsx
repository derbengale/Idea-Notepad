import { useIpcError } from "@/app/errors";
import { useUi } from "@/app/store";
import { useDefaultContextMenuPolicy } from "@/app/useDefaultContextMenuPolicy";
import { Header } from "@/components/app/Header";
import { SettingsOverlay } from "@/components/app/SettingsOverlay";
import { ShortcutsSheet } from "@/components/app/ShortcutsSheet";
import { Button } from "@/components/ui/button";
import { useToast } from "@/components/ui/toast";
import { EntryDetail } from "@/features/entries/EntryDetail";
import { EntryList } from "@/features/entries/EntryList";
import { LinkDialog } from "@/features/entries/LinkDialog";
import { Sidebar } from "@/features/entries/Sidebar";
import { ProjectDialog } from "@/features/projects/ProjectDialog";
import { SchemaEditor } from "@/features/schema/SchemaEditor";
import { CommandPalette } from "@/features/search/CommandPalette";
import { ShareDialog } from "@/features/share/ShareDialog";
import { SketchOverlay } from "@/features/sketch/SketchOverlay";
import { resolveLanguage } from "@/i18n";
import {
    appInfo,
    entryCreate,
    entryDelete,
    entryDuplicate,
    entryList,
    entryMoveProject,
    entryRestore,
    exportEntryMarkdown,
    linkCreate,
    projectList,
    schemaList,
    settingsGet,
    settingsSet,
    syncNow,
    syncStatus,
    tagList,
} from "@/lib/ipc";
import type { EntryQuery, Settings } from "@/lib/ipc/types";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { listen } from "@tauri-apps/api/event";
import { save as saveDialog } from "@tauri-apps/plugin-dialog";
import { Loader2, Plus, RefreshCw, TriangleAlert } from "lucide-react";
import {
    useCallback,
    useEffect,
    useMemo,
    useRef,
    useState,
    type PointerEvent as ReactPointerEvent,
} from "react";
import { useTranslation } from "react-i18next";

export const defaultSettings: Settings = {
  theme: "system",
  language: "system",
  defaultSchemaId: null,
  lastSchemaId: null,
  editorMode: "split",
  editorFontSize: 14,
  spellCheck: true,
  loadRemoteImages: false,
  allowRawHtml: false,
  density: "comfortable",
  sidebarWidth: 260,
  listWidth: 400,
  detailWidth: 0,
};

export default function App() {
  const { t, i18n } = useTranslation(["common", "entries"]);
  const queryClient = useQueryClient();
  const showError = useIpcError();
  const { push } = useToast();
  const ui = useUi();
  const searchRef = useRef<HTMLInputElement>(null);
  const autoSelected = useRef(false);
  const [linkSourceId, setLinkSourceId] = useState<string | null>(null);

  useDefaultContextMenuPolicy();

  const infoQuery = useQuery({ queryKey: ["appInfo"], queryFn: appInfo });
  const settingsQuery = useQuery({ queryKey: ["settings"], queryFn: settingsGet });
  const schemasQuery = useQuery({ queryKey: ["schemas"], queryFn: schemaList });
  const projectsQuery = useQuery({ queryKey: ["projects"], queryFn: projectList });
  const tagsQuery = useQuery({ queryKey: ["tags"], queryFn: tagList });
  const syncQuery = useQuery({
    queryKey: ["sync"],
    queryFn: syncStatus,
    refetchInterval: 15_000,
  });

  const settings = settingsQuery.data ?? defaultSettings;
  const schemas = useMemo(() => schemasQuery.data ?? [], [schemasQuery.data]);
  const projects = useMemo(() => projectsQuery.data ?? [], [projectsQuery.data]);
  const tags = useMemo(() => tagsQuery.data ?? [], [tagsQuery.data]);

  const entryQuery = useMemo<EntryQuery>(
    () => ({
      search: ui.search || undefined,
      schemaIds: ui.schemaFilter ? [ui.schemaFilter] : [],
      projectId: ui.projectFilter ?? undefined,
      tagIds: ui.tagFilters,
      sort: ui.sort,
      limit: 500,
      includeDeleted: ui.trash,
    }),
    [ui.search, ui.schemaFilter, ui.projectFilter, ui.tagFilters, ui.sort, ui.trash],
  );

  const entriesQuery = useQuery({
    queryKey: ["entries", entryQuery],
    queryFn: () => entryList(entryQuery),
  });
  const items = useMemo(() => entriesQuery.data?.items ?? [], [entriesQuery.data]);
  const selectedItem = items.find((item) => item.id === ui.selectedEntryId);

  useEffect(() => {
    if (ui.selectedEntryId || ui.trash || autoSelected.current || items.length === 0) {
      return;
    }
    autoSelected.current = true;
    ui.navigateEntry(items[0]?.id ?? "");
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [items, ui.trash, ui.selectedEntryId]);

  useEffect(() => {
    i18n.changeLanguage(resolveLanguage(settings.language));
  }, [settings.language, i18n]);

  useEffect(() => {
    const media = window.matchMedia("(prefers-color-scheme: dark)");
    const apply = () => {
      const theme =
        settings.theme === "system" ? (media.matches ? "dark" : "light") : settings.theme;
      document.documentElement.dataset.theme = theme;
      document.documentElement.style.colorScheme = theme;
    };
    apply();
    media.addEventListener("change", apply);
    return () => media.removeEventListener("change", apply);
  }, [settings.theme]);

  const editorModeSynced = useRef(false);
  useEffect(() => {
    if (!settingsQuery.data || editorModeSynced.current) {
      return;
    }
    editorModeSynced.current = true;
    ui.setEditorMode(settingsQuery.data.editorMode);
    ui.setDensity(settingsQuery.data.density);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [settingsQuery.data]);

  useEffect(() => {
    const subscriptions = [
      listen("sync://status", () => {
        void queryClient.invalidateQueries({ queryKey: ["sync"] });
      }),
      listen("auth://changed", () => {
        void queryClient.invalidateQueries({ queryKey: ["auth"] });
      }),
      listen("workspace://changed", () => {
        void queryClient.invalidateQueries();
      }),
      listen<{ entryId: string; conflictEntryId: string; title: string }>(
        "sync://conflict",
        (event) => {
          push(t("common:toast.error") + " · " + event.payload.title, {
            tone: "error",
            duration: 0,
          });
          void queryClient.invalidateQueries();
        },
      ),
    ];
    return () => {
      for (const subscription of subscriptions) {
        void subscription.then((unlisten) => unlisten());
      }
    };
  }, [push, queryClient, t]);

  const newNote = useCallback(
    async (schemaId?: string) => {
      const target =
        schemaId ??
        ui.schemaFilter ??
        settings.lastSchemaId ??
        settings.defaultSchemaId ??
        schemas[0]?.id;
      if (!target) {
        ui.openSchemaEditor(null);
        return;
      }
      try {
        const entry = await entryCreate({
          schemaId: target,
          projectId: ui.projectFilter && ui.projectFilter !== "inbox" ? ui.projectFilter : null,
          title: t("common:actions.untitled"),
          bodyMd: "",
          values: {},
          tagIds: [],
          links: [],
        });
        void queryClient.invalidateQueries({ queryKey: ["entries"] });
        void queryClient.invalidateQueries({ queryKey: ["stats"] });
        ui.navigateEntry(entry.id);
      } catch (error) {
        showError(error);
      }
    },
    [settings, schemas, ui, queryClient, showError, t],
  );

  const deleteEntry = useCallback(
    async (id: string) => {
      const item = items.find((candidate) => candidate.id === id);
      if (!window.confirm(t("entries:detail.deleteMessage", { title: item?.title ?? "" }))) {
        return;
      }
      try {
        await entryDelete(id);
        if (ui.selectedEntryId === id) {
          ui.selectEntry(null);
        }
        void queryClient.invalidateQueries({ queryKey: ["entries"] });
        void queryClient.invalidateQueries({ queryKey: ["stats"] });
        push(t("common:toast.deleted"), {
          tone: "info",
          duration: 10_000,
          actionLabel: t("common:actions.restore"),
          onAction: () => {
            void entryRestore(id)
              .then(() => queryClient.invalidateQueries())
              .catch(showError);
          },
        });
      } catch (error) {
        showError(error);
      }
    },
    [items, ui, queryClient, push, t, showError],
  );

  const deleteSelected = useCallback(async () => {
    const id = ui.selectedEntryId;
    if (!id) {
      return;
    }
    await deleteEntry(id);
  }, [ui.selectedEntryId, deleteEntry]);

  const duplicateEntry = useCallback(
    async (id: string) => {
      try {
        const copy = await entryDuplicate(id);
        void queryClient.invalidateQueries({ queryKey: ["entries"] });
        void queryClient.invalidateQueries({ queryKey: ["stats"] });
        ui.navigateEntry(copy.id);
      } catch (error) {
        showError(error);
      }
    },
    [queryClient, ui, showError],
  );

  const moveEntryProject = useCallback(
    async (id: string, projectId: string | null) => {
      try {
        await entryMoveProject(id, projectId);
        void queryClient.invalidateQueries({ queryKey: ["entries"] });
        void queryClient.invalidateQueries({ queryKey: ["entry", id] });
        void queryClient.invalidateQueries({ queryKey: ["stats"] });
        push(t("common:toast.moved"), { tone: "success" });
      } catch (error) {
        showError(error);
      }
    },
    [queryClient, push, t, showError],
  );

  const exportEntryById = useCallback(
    async (id: string) => {
      const item = items.find((candidate) => candidate.id === id);
      try {
        const target = await saveDialog({
          defaultPath: `${item?.title ?? t("common:actions.untitled")}.md`,
          filters: [{ name: "Markdown", extensions: ["md"] }],
        });
        if (!target) {
          return;
        }
        const result = await exportEntryMarkdown(id, target);
        push(`${t("entries:detail.export")}: ${result.path}`, { tone: "success" });
      } catch (error) {
        showError(error);
      }
    },
    [items, push, t, showError],
  );

  const createLink = useCallback(
    async (targetId: string) => {
      if (!linkSourceId) {
        return;
      }
      try {
        await linkCreate(linkSourceId, targetId);
        void queryClient.invalidateQueries({ queryKey: ["links", linkSourceId] });
        void queryClient.invalidateQueries({ queryKey: ["entry", linkSourceId] });
        push(t("common:toast.linked"), { tone: "success" });
        setLinkSourceId(null);
      } catch (error) {
        showError(error);
      }
    },
    [linkSourceId, queryClient, push, t, showError],
  );

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      const target = event.target as HTMLElement | null;
      const typing =
        target !== null &&
        (target.tagName === "INPUT" ||
          target.tagName === "TEXTAREA" ||
          target.tagName === "SELECT" ||
          target.isContentEditable);
      const mod = event.ctrlKey || event.metaKey;

      if (event.key === "Escape") {
        if (ui.shareMode) {
          ui.setShareMode(null);
        } else if (ui.paletteOpen) {
          ui.setPaletteOpen(false);
        } else if (ui.shortcutsOpen) {
          ui.setShortcutsOpen(false);
        } else if (ui.settingsOpen) {
          ui.closeSettings();
        } else if (ui.schemaEditorNew || ui.schemaEditorId) {
          ui.closeSchemaEditor();
        } else if (ui.search) {
          ui.setSearch("");
        }
        return;
      }

      if (typing) {
        return;
      }

      if (mod && event.shiftKey && event.key.toLowerCase() === "n") {
        event.preventDefault();
        ui.setPaletteOpen(true);
      } else if (mod && event.shiftKey && event.key.toLowerCase() === "s") {
        event.preventDefault();
        ui.openSchemaEditor(null);
      } else if (mod && (event.key.toLowerCase() === "k" || event.key.toLowerCase() === "p")) {
        event.preventDefault();
        ui.setPaletteOpen(true);
      } else if (mod && event.key.toLowerCase() === "n") {
        event.preventDefault();
        void newNote();
      } else if (mod && event.key.toLowerCase() === "f") {
        event.preventDefault();
        searchRef.current?.focus();
        searchRef.current?.select();
      } else if (mod && event.key.toLowerCase() === "e") {
        event.preventDefault();
        ui.setEditorMode(
          ui.editorMode === "source" ? "preview" : ui.editorMode === "preview" ? "split" : "source",
        );
      } else if (mod && event.key.toLowerCase() === "b") {
        event.preventDefault();
        ui.toggleSidebar();
      } else if (mod && event.key === "/") {
        event.preventDefault();
        ui.setShortcutsOpen(true);
      } else if (mod && /^[1-9]$/.test(event.key)) {
        const schema = schemas[Number(event.key) - 1];
        if (schema) {
          event.preventDefault();
          ui.setSchemaFilter(schema.id);
        }
      } else if (event.altKey && event.key === "ArrowLeft") {
        event.preventDefault();
        ui.back();
      } else if (event.altKey && event.key === "ArrowRight") {
        event.preventDefault();
        ui.forward();
      } else if (event.key === "Delete" && ui.selectedEntryId) {
        event.preventDefault();
        void deleteSelected();
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [ui, schemas, newNote, deleteSelected]);

  const [sidebarWidth, setSidebarWidth] = useState(settings.sidebarWidth);
  const [listWidth, setListWidth] = useState(settings.listWidth);
  const widthsReady = useRef(false);
  useEffect(() => {
    if (!widthsReady.current && settingsQuery.data) {
      widthsReady.current = true;
      setSidebarWidth(settingsQuery.data.sidebarWidth);
      setListWidth(settingsQuery.data.listWidth);
    }
  }, [settingsQuery.data]);

  const beginResize =
    (target: "sidebar" | "list") => (event: ReactPointerEvent<HTMLDivElement>) => {
      event.preventDefault();
      const startX = event.clientX;
      const startSidebar = sidebarWidth;
      const startList = listWidth;
      let latestSidebar = startSidebar;
      let latestList = startList;
      const onMove = (move: PointerEvent) => {
        const delta = move.clientX - startX;
        if (target === "sidebar") {
          latestSidebar = Math.min(380, Math.max(200, startSidebar + delta));
          setSidebarWidth(latestSidebar);
        } else {
          latestList = Math.min(680, Math.max(280, startList + delta));
          setListWidth(latestList);
        }
      };
      const onUp = () => {
        window.removeEventListener("pointermove", onMove);
        window.removeEventListener("pointerup", onUp);
        const current = settingsQuery.data;
        if (!current) {
          return;
        }
        void settingsSet({
          ...current,
          sidebarWidth: latestSidebar,
          listWidth: latestList,
        })
          .then((value) => queryClient.setQueryData(["settings"], value))
          .catch(() => undefined);
      };
      window.addEventListener("pointermove", onMove);
      window.addEventListener("pointerup", onUp);
    };

  if (infoQuery.isError || settingsQuery.isError || schemasQuery.isError) {
    return (
      <div className="flex h-full flex-col items-center justify-center gap-3 bg-[var(--bg)] p-8">
        <TriangleAlert size={24} className="text-[var(--danger)]" />
        <p className="text-[14px]">{t("common:states.error")}</p>
        <Button variant="primary" onClick={() => window.location.reload()}>
          {t("common:actions.retry")}
        </Button>
      </div>
    );
  }

  if (!infoQuery.data || !settingsQuery.data) {
    return (
      <div className="flex h-full items-center justify-center bg-[var(--bg)] text-[var(--text-muted)]">
        <Loader2 className="animate-spin" size={20} />
      </div>
    );
  }

  const offline = syncQuery.data?.state === "offline";
  const syncError = syncQuery.data?.state === "error" ? syncQuery.data.lastError : null;

  return (
    <div className="app-shell">
      <Header selected={selectedItem} schemas={schemas} projects={projects} searchRef={searchRef} />

      {offline ? (
        <div className="flex items-center gap-2 border-b border-[var(--warning)] bg-[var(--warning-soft)] px-4 py-1.5 text-[12px] text-[var(--warning)]">
          <TriangleAlert size={13} />
          {t("common:states.offline")}
        </div>
      ) : null}
      {syncError ? (
        <div className="flex items-center gap-2 border-b border-[var(--danger)] bg-[var(--danger-soft)] px-4 py-1.5 text-[12px] text-[var(--danger)]">
          <TriangleAlert size={13} />
          <span className="min-w-0 flex-1 truncate">{syncError.message}</span>
          <Button
            size="sm"
            className="h-6 px-1.5 text-[11px]"
            onClick={() =>
              void syncNow()
                .then(() => queryClient.invalidateQueries())
                .catch(showError)
            }
          >
            <RefreshCw size={11} />
            {t("common:actions.retry")}
          </Button>
        </div>
      ) : null}

      <div
        className="workspace"
        style={{
          gridTemplateColumns: ui.sidebarCollapsed
            ? `${listWidth}px 4px minmax(0, 1fr)`
            : `${sidebarWidth}px 4px ${listWidth}px 4px minmax(0, 1fr)`,
        }}
      >
        {!ui.sidebarCollapsed ? (
          <>
            <aside className="region">
              <Sidebar
                schemas={schemas}
                projects={projects}
                tags={tags}
                entries={items}
                version={infoQuery.data.version}
                defaultSchemaId={settings.lastSchemaId ?? settings.defaultSchemaId}
                onNewNote={(schemaId) => void newNote(schemaId)}
                onNewProject={() => ui.setProjectDialogOpen(true)}
              />
            </aside>
            <div
              role="separator"
              aria-orientation="vertical"
              className="cursor-col-resize bg-[var(--border)] hover:bg-[var(--accent)]"
              onPointerDown={beginResize("sidebar")}
            />
          </>
        ) : null}

        <section className="region">
          <EntryList
            items={items}
            total={entriesQuery.data?.total ?? 0}
            loading={entriesQuery.isLoading}
            schemas={schemas}
            projects={projects}
            tags={tags}
            onSelect={ui.navigateEntry}
            onEdit={(id) => ui.requestEdit(id)}
            onRestore={(id) => {
              void entryRestore(id)
                .then(() => queryClient.invalidateQueries())
                .catch(showError);
            }}
            onNewNote={() => void newNote()}
            onDuplicate={(id) => void duplicateEntry(id)}
            onDelete={(id) => void deleteEntry(id)}
            onMoveProject={(id, projectId) => void moveEntryProject(id, projectId)}
            onLink={(id) => setLinkSourceId(id)}
            onExport={(id) => void exportEntryById(id)}
          />
        </section>

        <div
          role="separator"
          aria-orientation="vertical"
          className="cursor-col-resize bg-[var(--border)] hover:bg-[var(--accent)]"
          onPointerDown={beginResize("list")}
        />

        <article className="region min-w-0">
          {ui.selectedEntryId ? (
            <EntryDetail
              key={ui.selectedEntryId}
              entryId={ui.selectedEntryId}
              schemas={schemas}
              projects={projects}
              tags={tags}
              appInfo={infoQuery.data}
              settings={settings}
            />
          ) : (
            <div className="relative flex flex-1 flex-col items-center justify-center gap-5 overflow-hidden p-8 text-center">
              <div className="art-rise relative flex w-[440px] max-w-[88%] items-center justify-center">
                <span aria-hidden="true" className="no-selection-glow absolute inset-0" />
                <img
                  src="/art/no-selection.png"
                  alt=""
                  aria-hidden="true"
                  draggable={false}
                  className="pointer-events-none relative w-full select-none"
                />
              </div>
              <div className="art-rise flex flex-col items-center gap-3 [animation-delay:90ms]">
                <p className="max-w-[300px] text-[14px] leading-relaxed text-[var(--text-muted)]">
                  {t("common:states.noSelection")}
                </p>
                <Button variant="primary" size="sm" onClick={() => void newNote()}>
                  <Plus size={14} />
                  {t("common:actions.newNote")}
                </Button>
              </div>
            </div>
          )}
        </article>
      </div>

      <SettingsOverlay />
      <SchemaEditor />
      <CommandPalette />
      <ProjectDialog open={ui.projectDialogOpen} onClose={() => ui.setProjectDialogOpen(false)} />
      <LinkDialog
        open={linkSourceId !== null}
        source={items.find((item) => item.id === linkSourceId) ?? null}
        entries={items}
        onClose={() => setLinkSourceId(null)}
        onLink={(targetId) => void createLink(targetId)}
      />
      <ShareDialog projects={projects} />
      <ShortcutsSheet />
      {ui.sketchId ? (
        <SketchOverlay
          sketchId={ui.sketchId}
          onClose={() => {
            ui.setSketchId(null);
            void queryClient.invalidateQueries({ queryKey: ["entry"] });
            void queryClient.invalidateQueries({ queryKey: ["entries"] });
          }}
        />
      ) : null}
    </div>
  );
}
