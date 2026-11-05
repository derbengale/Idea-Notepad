import { create } from "zustand";

export interface SortSpec {
  key: string;
  direction: "asc" | "desc";
}

interface UiState {
  selectedEntryId: string | null;
  schemaFilter: string | null;
  projectFilter: string | null;
  tagFilters: string[];
  search: string;
  trash: boolean;
  sort: SortSpec;
  density: "comfortable" | "compact";
  editorMode: "source" | "preview" | "split";
  sidebarCollapsed: boolean;
  settingsOpen: boolean;
  settingsSection: string;
  schemaEditorId: string | null;
  schemaEditorNew: boolean;
  paletteOpen: boolean;
  shortcutsOpen: boolean;
  shareMode: "export" | "import" | null;
  sketchId: string | null;
  projectDialogOpen: boolean;
  editRequestId: string | null;
  history: string[];
  historyIndex: number;
  selectEntry: (id: string | null) => void;
  requestEdit: (id: string) => void;
  clearEditRequest: () => void;
  navigateEntry: (id: string) => void;
  back: () => void;
  forward: () => void;
  setSchemaFilter: (id: string | null) => void;
  setProjectFilter: (id: string | null) => void;
  toggleTag: (id: string) => void;
  setSearch: (value: string) => void;
  setTrash: (value: boolean) => void;
  setSort: (sort: SortSpec) => void;
  setDensity: (density: "comfortable" | "compact") => void;
  setEditorMode: (mode: "source" | "preview" | "split") => void;
  toggleSidebar: () => void;
  openSettings: (section?: string) => void;
  closeSettings: () => void;
  openSchemaEditor: (id: string | null) => void;
  closeSchemaEditor: () => void;
  setPaletteOpen: (value: boolean) => void;
  setShortcutsOpen: (value: boolean) => void;
  setShareMode: (mode: "export" | "import" | null) => void;
  setSketchId: (id: string | null) => void;
  setProjectDialogOpen: (open: boolean) => void;
  clearFilters: () => void;
}

export const useUi = create<UiState>((set, get) => ({
  selectedEntryId: null,
  schemaFilter: null,
  projectFilter: null,
  tagFilters: [],
  search: "",
  trash: false,
  sort: { key: "updated", direction: "desc" },
  density: "comfortable",
  editorMode: "split",
  sidebarCollapsed: false,
  settingsOpen: false,
  settingsSection: "general",
  schemaEditorId: null,
  schemaEditorNew: false,
  paletteOpen: false,
  shortcutsOpen: false,
  shareMode: null,
  sketchId: null,
  projectDialogOpen: false,
  editRequestId: null,
  history: [],
  historyIndex: -1,
  selectEntry: (id) => set({ selectedEntryId: id, editRequestId: null }),
  requestEdit: (id) => set({ selectedEntryId: id, editRequestId: id }),
  clearEditRequest: () => set({ editRequestId: null }),
  navigateEntry: (id) => {
    const { history, historyIndex } = get();
    const next = history.slice(0, historyIndex + 1);
    next.push(id);
    set({ history: next, historyIndex: next.length - 1, selectedEntryId: id, editRequestId: null });
  },
  back: () => {
    const { history, historyIndex } = get();
    if (historyIndex <= 0) {
      return;
    }
    const index = historyIndex - 1;
    const id = history[index];
    if (id) {
      set({ historyIndex: index, selectedEntryId: id });
    }
  },
  forward: () => {
    const { history, historyIndex } = get();
    if (historyIndex >= history.length - 1) {
      return;
    }
    const index = historyIndex + 1;
    const id = history[index];
    if (id) {
      set({ historyIndex: index, selectedEntryId: id });
    }
  },
  setSchemaFilter: (id) => set({ schemaFilter: id, selectedEntryId: null }),
  setProjectFilter: (id) => set({ projectFilter: id, selectedEntryId: null }),
  toggleTag: (id) =>
    set((state) => ({
      tagFilters: state.tagFilters.includes(id)
        ? state.tagFilters.filter((tag) => tag !== id)
        : [...state.tagFilters, id],
      selectedEntryId: null,
    })),
  setSearch: (value) => set({ search: value }),
  setTrash: (value) => set({ trash: value, selectedEntryId: null }),
  setSort: (sort) => set({ sort }),
  setDensity: (density) => set({ density }),
  setEditorMode: (editorMode) => set({ editorMode }),
  toggleSidebar: () => set((state) => ({ sidebarCollapsed: !state.sidebarCollapsed })),
  openSettings: (section) => set({ settingsOpen: true, settingsSection: section ?? "general" }),
  closeSettings: () => set({ settingsOpen: false }),
  openSchemaEditor: (id) =>
    set({ schemaEditorId: id, schemaEditorNew: id === null, paletteOpen: false }),
  closeSchemaEditor: () => set({ schemaEditorId: null, schemaEditorNew: false }),
  setPaletteOpen: (paletteOpen) => set({ paletteOpen }),
  setShortcutsOpen: (shortcutsOpen) => set({ shortcutsOpen }),
  setShareMode: (shareMode) => set({ shareMode }),
  setSketchId: (sketchId) => set({ sketchId }),
  setProjectDialogOpen: (projectDialogOpen) => set({ projectDialogOpen }),
  clearFilters: () =>
    set({
      schemaFilter: null,
      projectFilter: null,
      tagFilters: [],
      search: "",
      trash: false,
    }),
}));
