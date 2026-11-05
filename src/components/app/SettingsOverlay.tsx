import { useIpcError } from "@/app/errors";
import { useUi } from "@/app/store";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Select } from "@/components/ui/select";
import { useToast } from "@/components/ui/toast";
import {
    appInfo,
    authSignIn,
    authSignOut,
    authStatus,
    diagnostics,
    openDataDir,
    rebuildIndex,
    schemaList,
    settingsGet,
    settingsSet,
    syncNow,
    syncPause,
    syncResume,
    syncStatus,
    workspaceStats,
} from "@/lib/ipc";
import type { Settings } from "@/lib/ipc/types";
import { cn } from "@/lib/utils/cn";
import { formatDate, formatRelative } from "@/lib/utils/format";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { ClipboardCopy, FolderOpen, RefreshCw, Save, X } from "lucide-react";
import { useEffect, useState, type ReactNode } from "react";
import { useTranslation } from "react-i18next";

const sections = ["general", "editor", "sync", "data", "diagnostics"] as const;

export function SettingsOverlay() {
  const { t, i18n } = useTranslation(["settings", "common", "sync", "account"]);
  const queryClient = useQueryClient();
  const showError = useIpcError();
  const { push } = useToast();
  const { settingsOpen, settingsSection, closeSettings, openSettings, setShareMode } = useUi();
  const [draft, setDraft] = useState<Settings | null>(null);

  const settings = useQuery({ queryKey: ["settings"], queryFn: settingsGet });
  const schemas = useQuery({ queryKey: ["schemas"], queryFn: schemaList, enabled: settingsOpen });
  const info = useQuery({ queryKey: ["appInfo"], queryFn: appInfo, enabled: settingsOpen });
  const stats = useQuery({ queryKey: ["stats"], queryFn: workspaceStats, enabled: settingsOpen });
  const diag = useQuery({ queryKey: ["diagnostics"], queryFn: diagnostics, enabled: settingsOpen });
  const auth = useQuery({ queryKey: ["auth"], queryFn: authStatus, enabled: settingsOpen });
  const sync = useQuery({ queryKey: ["sync"], queryFn: syncStatus, enabled: settingsOpen });
  const [rebuilding, setRebuilding] = useState(false);

  useEffect(() => {
    if (settings.data) {
      setDraft(settings.data);
    }
  }, [settings.data]);

  const save = useMutation({
    mutationFn: (value: Settings) => settingsSet(value),
    onSuccess: (value) => {
      queryClient.setQueryData(["settings"], value);
      void queryClient.invalidateQueries({ queryKey: ["auth"] });
      push(t("settings:saved"), { tone: "success" });
    },
    onError: showError,
  });

  const signIn = useMutation({
    mutationFn: authSignIn,
    onSuccess: () => queryClient.invalidateQueries(),
    onError: showError,
  });
  const signOut = useMutation({
    mutationFn: () => authSignOut(false),
    onSuccess: () => queryClient.invalidateQueries(),
    onError: showError,
  });

  if (!settingsOpen || !draft) {
    return null;
  }

  const update = (patch: Partial<Settings>) => setDraft({ ...draft, ...patch });

  const handleRebuild = async () => {
    try {
      setRebuilding(true);
      const report = await rebuildIndex();
      push(t("settings:rebuildDone", { notes: report.notes, ms: report.durationMs }), {
        tone: "success",
      });
      if (report.warnings.length > 0) {
        push(t("settings:rebuildWarnings", { count: report.warnings.length }), {
          tone: "error",
          duration: 0,
        });
      }
      void queryClient.invalidateQueries();
    } catch (error) {
      showError(error);
    } finally {
      setRebuilding(false);
    }
  };

  const copyDiagnostics = async () => {
    const payload = JSON.stringify(
      {
        info: info.data,
        diagnostics: diag.data,
        stats: stats.data,
        settings: draft,
      },
      null,
      2,
    );
    await navigator.clipboard.writeText(payload);
    push(t("settings:copied"), { tone: "success" });
  };

  const themeOptions = ["system", "light", "dark"] as const;
  const languageOptions = ["system", "en", "de"] as const;

  return (
    <div className="fixed inset-0 z-40 flex bg-[var(--bg)]">
      <aside className="flex w-[220px] shrink-0 flex-col border-r border-[var(--border)] bg-[var(--bg-elevated)] p-3">
        <div className="mb-3 flex items-center justify-between">
          <h1 className="text-[15px] font-semibold">{t("settings:title")}</h1>
          <Button
            variant="ghost"
            size="icon"
            onClick={closeSettings}
            aria-label={t("common:actions.close")}
          >
            <X size={16} />
          </Button>
        </div>
        <nav className="space-y-0.5">
          {sections.map((section) => (
            <button
              key={section}
              type="button"
              onClick={() => openSettings(section)}
              className={cn(
                "w-full rounded-[var(--r-sm)] px-2 py-1.5 text-left text-[12.5px]",
                settingsSection === section
                  ? "bg-[var(--accent-soft)] font-medium text-[var(--accent)]"
                  : "text-[var(--text)] hover:bg-[var(--bg-subtle)]",
              )}
            >
              {t(`settings:${section}`)}
            </button>
          ))}
        </nav>
        <div className="mt-auto flex flex-col gap-1">
          <Button variant="primary" onClick={() => save.mutate(draft)} disabled={save.isPending}>
            <Save size={14} />
            {t("settings:save")}
          </Button>
        </div>
      </aside>

      <div className="min-h-0 flex-1 overflow-auto p-6">
        <div className="mx-auto max-w-[720px] space-y-5">
          {settingsSection === "general" ? (
            <>
              <Row label={t("settings:theme")}>
                <Select
                  value={draft.theme}
                  onChange={(event) => update({ theme: event.target.value as Settings["theme"] })}
                >
                  {themeOptions.map((option) => (
                    <option key={option} value={option}>
                      {t(`common:theme.${option}`)}
                    </option>
                  ))}
                </Select>
              </Row>
              <Row label={t("settings:language")}>
                <Select
                  value={draft.language}
                  onChange={(event) =>
                    update({ language: event.target.value as Settings["language"] })
                  }
                >
                  {languageOptions.map((option) => (
                    <option key={option} value={option}>
                      {t(`common:language.${option}`)}
                    </option>
                  ))}
                </Select>
              </Row>
              <Row label={t("settings:defaultSchema")}>
                <Select
                  value={draft.defaultSchemaId ?? ""}
                  onChange={(event) => update({ defaultSchemaId: event.target.value || null })}
                >
                  <option value="">{t("common:actions.untitled")}</option>
                  {(schemas.data ?? []).map((schema) => (
                    <option key={schema.id} value={schema.id}>
                      {schema.name}
                    </option>
                  ))}
                </Select>
              </Row>
              <Row label={t("settings:density")}>
                <Select
                  value={draft.density}
                  onChange={(event) =>
                    update({ density: event.target.value as Settings["density"] })
                  }
                >
                  <option value="comfortable">{t("common:labels.density")} — comfortable</option>
                  <option value="compact">{t("common:labels.density")} — compact</option>
                </Select>
              </Row>
            </>
          ) : null}

          {settingsSection === "editor" ? (
            <>
              <Row label={t("settings:editorMode")}>
                <Select
                  value={draft.editorMode}
                  onChange={(event) =>
                    update({ editorMode: event.target.value as Settings["editorMode"] })
                  }
                >
                  {(["source", "preview", "split"] as const).map((mode) => (
                    <option key={mode} value={mode}>
                      {t(`settings:mode.${mode}`)}
                    </option>
                  ))}
                </Select>
              </Row>
              <Row label={t("settings:fontSize")}>
                <Input
                  type="number"
                  min={10}
                  max={24}
                  value={draft.editorFontSize}
                  onChange={(event) => update({ editorFontSize: Number(event.target.value) })}
                />
              </Row>
              <Toggle
                label={t("settings:spellCheck")}
                checked={draft.spellCheck}
                onChange={(spellCheck) => update({ spellCheck })}
              />
              <Toggle
                label={t("settings:loadRemoteImages")}
                checked={draft.loadRemoteImages}
                onChange={(loadRemoteImages) => update({ loadRemoteImages })}
              />
              <Toggle
                label={t("settings:allowRawHtml")}
                checked={draft.allowRawHtml}
                onChange={(allowRawHtml) => update({ allowRawHtml })}
              />
            </>
          ) : null}

          {settingsSection === "sync" ? (
            <>
              <div className="rounded-[var(--r-md)] border border-[var(--border)] p-3">
                <p className="text-[13px] font-medium">
                  {auth.data?.signedIn
                    ? t("account:signedInAs", { email: auth.data.email ?? "" })
                    : t("account:notSignedIn")}
                </p>
                <p className="mb-2 text-[11.5px] text-[var(--text-muted)]">
                  {t("sync:privateFolder")}
                </p>
                {auth.data?.signedIn ? (
                  <div className="flex flex-wrap gap-1">
                    <Button size="sm" onClick={() => void syncNow().catch(showError)}>
                      <RefreshCw size={12} />
                      {t("sync:syncNow")}
                    </Button>
                    <Button size="sm" onClick={() => void syncPause().catch(showError)}>
                      {t("sync:pause")}
                    </Button>
                    <Button size="sm" onClick={() => void syncResume().catch(showError)}>
                      {t("sync:resume")}
                    </Button>
                    <Button size="sm" variant="danger" onClick={() => signOut.mutate()}>
                      {t("common:actions.signOut")}
                    </Button>
                  </div>
                ) : (
                  <Button
                    variant="primary"
                    size="sm"
                    disabled={!auth.data?.hasClientId || signIn.isPending}
                    onClick={() => signIn.mutate()}
                  >
                    {t("account:signIn")}
                  </Button>
                )}
                <p className="mt-2 text-[11px] text-[var(--text-muted)]">
                  {sync.data?.lastSyncAt
                    ? t("sync:lastSync", {
                        when: formatRelative(sync.data.lastSyncAt, i18n.language),
                      })
                    : t("sync:never")}
                </p>
              </div>
            </>
          ) : null}

          {settingsSection === "data" ? (
            <>
              <PathRow label={t("settings:dataDir")} value={info.data?.dataDir ?? ""} />
              <PathRow label={t("settings:dbPath")} value={info.data?.dbPath ?? ""} />
              <PathRow label={t("settings:mirrorPath")} value={info.data?.mirrorPath ?? ""} />
              <PathRow label={t("settings:logPath")} value={info.data?.logPath ?? ""} />
              <div className="flex flex-wrap gap-1">
                <Button onClick={() => void openDataDir().catch(showError)}>
                  <FolderOpen size={13} />
                  {t("settings:openDataFolder")}
                </Button>
                <Button onClick={() => void handleRebuild()} disabled={rebuilding}>
                  <RefreshCw size={13} className={rebuilding ? "animate-spin" : undefined} />
                  {rebuilding ? t("settings:rebuilding") : t("settings:rebuildIndex")}
                </Button>
                <Button
                  onClick={() => {
                    closeSettings();
                    setShareMode("export");
                  }}
                >
                  {t("settings:exportWorkspace")}
                </Button>
                <Button
                  onClick={() => {
                    closeSettings();
                    setShareMode("import");
                  }}
                >
                  {t("settings:importBundle")}
                </Button>
              </div>
              <p className="text-[11.5px] text-[var(--text-muted)]">
                {t("settings:trashRetention")}
              </p>
            </>
          ) : null}

          {settingsSection === "diagnostics" ? (
            <>
              <PathRow
                label={t("settings:appVersion")}
                value={String(diag.data?.appVersion ?? "")}
              />
              <PathRow
                label={t("settings:tauriVersion")}
                value={String(diag.data?.tauriVersion ?? "")}
              />
              <PathRow
                label={t("settings:migrations")}
                value={String(diag.data?.migrationVersion ?? "")}
              />
              <p className="text-[12.5px] text-[var(--text-muted)]">
                {t("settings:counts", {
                  entries: stats.data?.entries ?? 0,
                  projects: stats.data?.projects ?? 0,
                  schemas: stats.data?.schemas ?? 0,
                  attachments: stats.data?.attachments ?? 0,
                  trash: stats.data?.trash ?? 0,
                })}
              </p>
              {stats.data?.lastSyncAt ? (
                <p className="text-[11.5px] text-[var(--text-muted)]">
                  {formatDate(stats.data.lastSyncAt, i18n.language, true)}
                </p>
              ) : null}
              <Button onClick={() => void copyDiagnostics()}>
                <ClipboardCopy size={13} />
                {t("settings:copyDiagnostics")}
              </Button>
            </>
          ) : null}
        </div>
      </div>
    </div>
  );
}

function Row({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div className="space-y-1">
      <span className="text-[12px] font-medium text-[var(--text-muted)]">{label}</span>
      {children}
    </div>
  );
}

function Toggle({
  label,
  checked,
  onChange,
}: {
  label: string;
  checked: boolean;
  onChange: (value: boolean) => void;
}) {
  return (
    <label className="flex items-center gap-2 text-[13px]">
      <input
        type="checkbox"
        className="h-4 w-4 accent-[var(--accent)]"
        checked={checked}
        onChange={(event) => onChange(event.target.checked)}
      />
      {label}
    </label>
  );
}

function PathRow({ label, value }: { label: string; value: string }) {
  return (
    <div>
      <span className="text-[12px] font-medium text-[var(--text-muted)]">{label}</span>
      <p className="break-all font-mono text-[11.5px]">{value}</p>
    </div>
  );
}
