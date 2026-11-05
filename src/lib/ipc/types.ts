export type Theme = "system" | "light" | "dark";
export type Language = "system" | "en" | "de";
export type EditorMode = "source" | "preview" | "split";

export interface Settings {
  theme: Theme;
  language: Language;
  defaultSchemaId: string | null;
  lastSchemaId: string | null;
  editorMode: EditorMode;
  editorFontSize: number;
  spellCheck: boolean;
  loadRemoteImages: boolean;
  allowRawHtml: boolean;
  density: "comfortable" | "compact";
  sidebarWidth: number;
  listWidth: number;
  detailWidth: number;
}

export interface AppInfo {
  version: string;
  platform: string;
  dataDir: string;
  dbPath: string;
  logPath: string;
  mirrorPath: string;
}

export interface TagDto {
  id: string;
  name: string;
  color: string;
}

export interface ProjectDto {
  id: string;
  name: string;
  description: string;
  color: string;
  icon: string;
  sortOrder: number;
  createdAt: string;
  updatedAt: string;
  noteCount: number;
}

export type FieldType =
  "text" | "longtext" | "number" | "scale" | "status" | "date" | "checkbox" | "tags" | "formula";

export interface StatusOption {
  key: string;
  label: string;
  color: string;
}

export interface FieldConfig {
  placeholder?: string;
  maxLength?: number;
  rows?: number;
  min?: number;
  max?: number;
  step?: number;
  precision?: number;
  suffix?: string;
  withTime?: boolean;
  default?: string | number | boolean | null;
  options?: StatusOption[];
  labels?: Record<string, string>;
  allowCreate?: boolean;
}

export interface FieldDef {
  id: string;
  key: string;
  label: string;
  type: FieldType;
  required: boolean;
  order: number;
  config: FieldConfig;
  expression: string | null;
  showInList: boolean;
}

export interface SchemaDto {
  id: string;
  key: string;
  name: string;
  icon: string;
  color: string;
  order: number;
  titleFieldId: string | null;
  subtitleFieldId: string | null;
  badgeFieldId: string | null;
  formatVersion: number;
  fields: FieldDef[];
}

export type FieldValue = string | number | boolean | null;

export interface EntryListItem {
  id: string;
  title: string;
  schemaId: string;
  schemaKey: string;
  projectId: string | null;
  tags: TagDto[];
  subtitle: string | null;
  badge: string | null;
  badgeColor: string | null;
  computed: Record<string, number | null>;
  createdAt: string;
  updatedAt: string;
  deletedAt: string | null;
  syncState: string;
}

export interface LinkItem {
  id: string | null;
  entryId: string;
  title: string;
  schemaKey: string;
  deleted: boolean;
}

export interface LinkBundle {
  links: LinkItem[];
  backlinks: LinkItem[];
  markdownLinks: LinkItem[];
  markdownBacklinks: LinkItem[];
}

export interface AttachmentDto {
  id: string;
  entryId: string;
  kind: "image" | "file" | "sketch";
  fileName: string;
  mimeType: string;
  byteSize: number;
  sha256: string;
  localPath: string;
  previewPath: string | null;
  createdAt: string;
}

export interface EntryDetail {
  id: string;
  schemaId: string;
  projectId: string | null;
  title: string;
  bodyMd: string;
  values: Record<string, FieldValue>;
  computed: Record<string, number | null>;
  tags: TagDto[];
  links: LinkBundle;
  attachments: AttachmentDto[];
  createdAt: string;
  updatedAt: string;
  deletedAt: string | null;
  syncState: string;
}

export interface EntryInput {
  id?: string;
  schemaId: string;
  projectId: string | null;
  title: string;
  bodyMd: string;
  values: Record<string, FieldValue>;
  tagIds: string[];
  links: string[];
  expectedUpdatedAt?: string | null;
}

export interface EntryQuery {
  search?: string;
  schemaIds?: string[];
  projectId?: string;
  tagIds?: string[];
  statusValue?: string;
  sort?: { key: string; direction: "asc" | "desc" };
  cursor?: string;
  limit?: number;
  includeDeleted?: boolean;
}

export interface EntryPage {
  items: EntryListItem[];
  total: number;
  nextCursor?: string | null;
}

export interface SearchHit {
  entryId: string;
  title: string;
  schemaKey: string;
  snippet: string;
  rank: number;
  updatedAt: string;
}

export interface SearchResult {
  items: SearchHit[];
  total: number;
}

export interface WorkspaceStats {
  entries: number;
  projects: number;
  schemas: number;
  attachments: number;
  trash: number;
  lastSyncAt: string | null;
}

export interface RebuildReport {
  notes: number;
  attachments: number;
  durationMs: number;
  warnings: string[];
}

export interface AuthStatus {
  signedIn: boolean;
  email: string | null;
  name: string | null;
  picture: string | null;
  hasClientId: boolean;
}

export interface AppErrorPayload {
  code: string;
  message: string;
  details?: unknown;
  retryable: boolean;
}

export interface SyncStatus {
  state: "idle" | "syncing" | "offline" | "paused" | "error" | "conflict";
  lastSyncAt: string | null;
  pendingOperations: number;
  conflictCount: number;
  lastError: AppErrorPayload | null;
}

export interface ExportResult {
  path: string;
  notes: number;
  attachments: number;
}

export interface BundlePreview {
  formatVersion: number;
  exportedAt: string;
  appVersion: string;
  scope: { type: string; projectId: string | null; name: string };
  notes: { total: number; new: number; updated: number; unchanged: number; conflicts: number };
  schemas: string[];
  schemaConflicts: string[];
  projects: string[];
  attachmentsTotal: number;
  attachmentsBytes: number;
  warnings: string[];
}

export interface ImportReport {
  created: number;
  updated: number;
  skipped: number;
  conflicts: number;
  attachmentsCopied: number;
  warnings: string[];
}

export interface LinkRow {
  id: string;
  fromId: string;
  toId: string;
}

export interface FormulaValidation {
  ok: boolean;
  error: string | null;
  references: string[];
}
