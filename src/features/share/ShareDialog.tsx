import { useIpcError } from "@/app/errors";
import { useUi } from "@/app/store";
import { Button } from "@/components/ui/button";
import { Dialog } from "@/components/ui/dialog";
import { Select } from "@/components/ui/select";
import { useToast } from "@/components/ui/toast";
import { bundleExport, bundleImport, bundleInspect } from "@/lib/ipc";
import type { BundlePreview, ImportReport, ProjectDto } from "@/lib/ipc/types";
import { formatBytes, formatDate } from "@/lib/utils/format";
import { useQueryClient } from "@tanstack/react-query";
import { open as openDialog, save as saveDialog } from "@tauri-apps/plugin-dialog";
import { useState } from "react";
import { useTranslation } from "react-i18next";

export interface ShareDialogProps {
  projects: ProjectDto[];
}

export function ShareDialog({ projects }: ShareDialogProps) {
  const { t, i18n } = useTranslation(["share", "common"]);
  const queryClient = useQueryClient();
  const { push } = useToast();
  const showError = useIpcError();
  const { shareMode, setShareMode, projectFilter } = useUi();
  const [scope, setScope] = useState<"workspace" | "project">("workspace");
  const [projectId, setProjectId] = useState(projectFilter ?? projects[0]?.id ?? "");
  const [sourcePath, setSourcePath] = useState<string | null>(null);
  const [preview, setPreview] = useState<BundlePreview | null>(null);
  const [mode, setMode] = useState<"merge" | "keepLocal" | "keepIncoming">("merge");
  const [busy, setBusy] = useState(false);

  const close = () => {
    setShareMode(null);
    setPreview(null);
    setSourcePath(null);
  };

  const handleExport = async () => {
    try {
      setBusy(true);
      const target = await saveDialog({
        defaultPath: "better-notepad.bnpkg",
        filters: [{ name: "Better Notepad bundle", extensions: ["bnpkg"] }],
      });
      if (!target) {
        return;
      }
      const result = await bundleExport(scope, scope === "project" ? projectId : null, target);
      push(t("share:export.done", { path: result.path }), { tone: "success" });
      close();
    } catch (error) {
      showError(error);
    } finally {
      setBusy(false);
    }
  };

  const chooseImport = async () => {
    try {
      const path = await openDialog({
        multiple: false,
        directory: false,
        filters: [{ name: "Better Notepad bundle", extensions: ["bnpkg"] }],
      });
      if (typeof path !== "string") {
        return;
      }
      const inspected = await bundleInspect(path);
      setSourcePath(path);
      setPreview(inspected);
    } catch (error) {
      showError(error);
    }
  };

  const handleImport = async () => {
    if (!sourcePath) {
      return;
    }
    try {
      setBusy(true);
      const report: ImportReport = await bundleImport(sourcePath, mode);
      push(
        t("share:import.done", {
          created: report.created,
          updated: report.updated,
          skipped: report.skipped,
          conflicts: report.conflicts,
        }),
        { tone: "success" },
      );
      if (report.warnings.length > 0) {
        push(report.warnings.join(" · "), { tone: "error", duration: 0 });
      }
      void queryClient.invalidateQueries();
      close();
    } catch (error) {
      showError(error);
    } finally {
      setBusy(false);
    }
  };

  return (
    <Dialog
      open={shareMode !== null}
      title={shareMode === "import" ? t("share:import.title") : t("share:export.title")}
      onClose={close}
      width="min(660px, 94vw)"
      footer={
        shareMode === "import" ? (
          <>
            <Button onClick={close}>{t("common:actions.cancel")}</Button>
            <Button
              variant="primary"
              onClick={() => void handleImport()}
              disabled={!preview || busy}
            >
              {t("share:import.confirm")}
            </Button>
          </>
        ) : (
          <>
            <Button onClick={close}>{t("common:actions.cancel")}</Button>
            <Button
              variant="primary"
              onClick={() => void handleExport()}
              disabled={busy || (scope === "project" && !projectId)}
            >
              {t("share:export.choosePath")}
            </Button>
          </>
        )
      }
    >
      {shareMode === "export" ? (
        <div className="space-y-3">
          <p className="text-[12.5px] text-[var(--text-muted)]">{t("share:export.description")}</p>
          <label className="block">
            <span className="text-[12px] text-[var(--text-muted)]">{t("share:export.scope")}</span>
            <Select
              value={scope}
              onChange={(event) => setScope(event.target.value as "workspace" | "project")}
            >
              <option value="workspace">{t("share:export.workspace")}</option>
              <option value="project">{t("share:export.project")}</option>
            </Select>
          </label>
          {scope === "project" ? (
            <label className="block">
              <span className="text-[12px] text-[var(--text-muted)]">
                {t("common:labels.projects")}
              </span>
              <Select value={projectId} onChange={(event) => setProjectId(event.target.value)}>
                {projects.map((project) => (
                  <option key={project.id} value={project.id}>
                    {project.name}
                  </option>
                ))}
              </Select>
            </label>
          ) : null}
        </div>
      ) : (
        <div className="space-y-3">
          <Button onClick={() => void chooseImport()}>{t("share:import.chooseFile")}</Button>
          {preview ? (
            <>
              <div className="rounded-[var(--r-md)] border border-[var(--border)] p-3 text-[12.5px]">
                <p className="font-medium">
                  {preview.scope.name} · {preview.scope.type}
                </p>
                <p className="text-[11.5px] text-[var(--text-muted)]">
                  {formatDate(preview.exportedAt, i18n.language, true)} · v{preview.appVersion}
                </p>
                <p className="mt-2">
                  {t("share:import.summary", {
                    new: preview.notes.new,
                    updated: preview.notes.updated,
                    unchanged: preview.notes.unchanged,
                    conflicts: preview.notes.conflicts,
                  })}
                </p>
                <p className="mt-1 text-[11.5px] text-[var(--text-muted)]">
                  {t("share:import.attachments", {
                    count: preview.attachmentsTotal,
                    size: formatBytes(preview.attachmentsBytes, i18n.language),
                  })}
                </p>
                {preview.schemas.length > 0 ? (
                  <p className="mt-1 text-[11.5px] text-[var(--text-muted)]">
                    {t("share:import.schemas")}: {preview.schemas.join(", ")}
                  </p>
                ) : null}
                {preview.schemaConflicts.length > 0 ? (
                  <p className="mt-1 text-[11.5px] text-[var(--warning)]">
                    {preview.schemaConflicts.join(", ")}
                  </p>
                ) : null}
                {preview.warnings.length > 0 ? (
                  <div className="mt-2 text-[11.5px] text-[var(--warning)]">
                    <p className="font-medium">{t("share:import.warnings")}</p>
                    <ul className="list-disc pl-4">
                      {preview.warnings.slice(0, 6).map((warning) => (
                        <li key={warning}>{warning}</li>
                      ))}
                    </ul>
                  </div>
                ) : null}
              </div>
              <label className="block">
                <span className="text-[12px] text-[var(--text-muted)]">
                  {t("share:import.mode")}
                </span>
                <Select
                  value={mode}
                  onChange={(event) =>
                    setMode(event.target.value as "merge" | "keepLocal" | "keepIncoming")
                  }
                >
                  <option value="merge">{t("share:import.modeMerge")}</option>
                  <option value="keepLocal">{t("share:import.modeKeepLocal")}</option>
                  <option value="keepIncoming">{t("share:import.modeKeepIncoming")}</option>
                </Select>
              </label>
            </>
          ) : null}
        </div>
      )}
    </Dialog>
  );
}
