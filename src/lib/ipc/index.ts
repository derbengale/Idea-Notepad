import { invoke } from "@tauri-apps/api/core";
import type {
  AppErrorPayload,
  AppInfo,
  AttachmentDto,
  AuthStatus,
  BundlePreview,
  EntryDetail,
  EntryInput,
  EntryPage,
  EntryQuery,
  ExportResult,
  FieldValue,
  FormulaValidation,
  ImportReport,
  LinkBundle,
  LinkItem,
  LinkRow,
  ProjectDto,
  RebuildReport,
  SchemaDto,
  SearchResult,
  Settings,
  SyncStatus,
  TagDto,
  WorkspaceStats,
} from "./types";

export class IpcError extends Error {
  readonly code: string;
  readonly details?: unknown;
  readonly retryable: boolean;

  constructor(payload: AppErrorPayload) {
    super(payload.message);
    this.name = "IpcError";
    this.code = payload.code;
    this.details = payload.details;
    this.retryable = payload.retryable;
  }
}

function isErrorPayload(value: unknown): value is AppErrorPayload {
  return (
    typeof value === "object" &&
    value !== null &&
    "code" in value &&
    "message" in value &&
    typeof (value as { code: unknown }).code === "string"
  );
}

export async function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(command, args);
  } catch (error) {
    if (isErrorPayload(error)) {
      throw new IpcError(error);
    }
    throw new IpcError({
      code: "UNKNOWN",
      message: error instanceof Error ? error.message : String(error),
      retryable: false,
    });
  }
}

export const appInfo = () => call<AppInfo>("app_info");
export const openDataDir = () => call<void>("app_open_data_dir");
export const settingsGet = () => call<Settings>("settings_get");
export const settingsSet = (settings: Settings) => call<Settings>("settings_set", { settings });
export const diagnostics = () => call<Record<string, string | number>>("diagnostics");

export const workspaceStats = () => call<WorkspaceStats>("workspace_stats");
export const rebuildIndex = () => call<RebuildReport>("workspace_rebuild_index");

export const entryList = (query: EntryQuery) => call<EntryPage>("entry_list", { query });
export const entryGet = (id: string) => call<EntryDetail>("entry_get", { id });
export const entryCreate = (input: EntryInput) => call<EntryDetail>("entry_create", { input });
export const entryUpdate = (input: EntryInput) => call<EntryDetail>("entry_update", { input });
export const entryDelete = (id: string) => call<void>("entry_delete", { id });
export const entryRestore = (id: string) => call<EntryDetail>("entry_restore", { id });
export const entryDuplicate = (id: string) => call<EntryDetail>("entry_duplicate", { id });
export const entryMoveProject = (id: string, projectId: string | null) =>
  call<EntryDetail>("entry_move_project", { id, projectId });
export const entrySetTags = (id: string, tagIds: string[]) =>
  call<EntryDetail>("entry_set_tags", { id, tagIds });

export const linkCreate = (fromId: string, toId: string) =>
  call<LinkItem>("link_create", { fromId, toId });
export const linkDelete = (id: string) => call<void>("link_delete", { id });
export const linkList = (entryId: string) => call<LinkBundle>("link_list", { entryId });

export const schemaList = () => call<SchemaDto[]>("schema_list");
export const schemaCreate = (input: Partial<SchemaDto> & { name: string }) =>
  call<SchemaDto>("schema_create", { input });
export const schemaUpdate = (schema: SchemaDto) => call<SchemaDto>("schema_update", { schema });
export const schemaDelete = (id: string, reassignTo: string) =>
  call<{ movedEntries: number }>("schema_delete", { id, reassignTo });
export const schemaReorder = (ids: string[]) => call<SchemaDto[]>("schema_reorder", { ids });
export const schemaDuplicate = (id: string, name: string) =>
  call<SchemaDto>("schema_duplicate", { id, name });
export const formulaValidate = (expression: string, schemaId?: string) =>
  call<FormulaValidation>("formula_validate", { expression, schemaId: schemaId ?? null });
export const formulaEval = (schemaId: string, values: Record<string, FieldValue>) =>
  call<{ values: Record<string, number | null> }>("formula_eval", { schemaId, values });

export const projectList = () => call<ProjectDto[]>("project_list");
export const projectCreate = (input: {
  name: string;
  description?: string;
  color?: string;
  icon?: string;
}) => call<ProjectDto>("project_create", { input });
export const projectUpdate = (project: ProjectDto) =>
  call<ProjectDto>("project_update", { project });
export const projectDelete = (id: string) =>
  call<{ movedEntries: number }>("project_delete", { id });

export const tagList = () => call<TagDto[]>("tag_list");
export const tagCreate = (name: string, color?: string) =>
  call<TagDto>("tag_create", { name, color: color ?? null });
export const tagDelete = (id: string) => call<void>("tag_delete", { id });

export const attachmentList = (entryId: string) =>
  call<AttachmentDto[]>("attachment_list", { entryId });
export const attachmentAdd = (entryId: string, sourcePath: string) =>
  call<AttachmentDto>("attachment_add", { entryId, sourcePath });
export const attachmentDelete = (id: string) => call<void>("attachment_delete", { id });
export const attachmentOpen = (id: string) => call<void>("attachment_open", { id });
export const sketchCreate = (entryId: string) => call<AttachmentDto>("sketch_create", { entryId });
export const sketchLoad = (id: string) =>
  call<{ sceneJson: string; previewPngBase64: string | null }>("sketch_load", { id });
export const sketchSave = (id: string, sceneJson: string, previewPngBase64: string | null) =>
  call<AttachmentDto>("sketch_save", { id, sceneJson, previewPngBase64 });
export const sketchDelete = (id: string) => call<void>("sketch_delete", { id });

export const searchQuery = (text: string, limit = 50) =>
  call<SearchResult>("search_query", { text, limit });

export const exportEntryMarkdown = (id: string, targetPath: string) =>
  call<ExportResult>("export_entry_markdown", { id, targetPath });
export const bundleExport = (
  scope: "workspace" | "project",
  projectId: string | null,
  targetPath: string,
) => call<ExportResult>("bundle_export", { scope, projectId, targetPath });
export const bundleInspect = (sourcePath: string) =>
  call<BundlePreview>("bundle_inspect", { sourcePath });
export const bundleImport = (sourcePath: string, mode: "merge" | "keepLocal" | "keepIncoming") =>
  call<ImportReport>("bundle_import", { sourcePath, mode });

export const authStatus = () => call<AuthStatus>("auth_status");
export const authSignIn = () => call<AuthStatus>("auth_sign_in");
export const authSignOut = (deleteLocalData: boolean) =>
  call<void>("auth_sign_out", { deleteLocalData });

export const syncStatus = () => call<SyncStatus>("sync_status");
export const syncNow = () => call<SyncStatus>("sync_now");
export const syncPause = () => call<SyncStatus>("sync_pause");
export const syncResume = () => call<SyncStatus>("sync_resume");

export type { LinkRow };
