import { useIpcError } from "@/app/errors";
import { Button } from "@/components/ui/button";
import { Dialog } from "@/components/ui/dialog";
import { Input, Textarea } from "@/components/ui/input";
import { projectCreate, projectDelete, projectList } from "@/lib/ipc";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { Plus, Trash2 } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";

export function ProjectDialog({ open, onClose }: { open: boolean; onClose: () => void }) {
  const { t } = useTranslation(["projects", "common"]);
  const queryClient = useQueryClient();
  const showError = useIpcError();
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  const [color, setColor] = useState("#64748b");
  const [deleteId, setDeleteId] = useState<string | null>(null);
  const projects = useQuery({ queryKey: ["projects"], queryFn: projectList, enabled: open });

  const create = async () => {
    try {
      await projectCreate({ name, description, color });
      setName("");
      setDescription("");
      void queryClient.invalidateQueries({ queryKey: ["projects"] });
      void queryClient.invalidateQueries({ queryKey: ["entry"] });
      onClose();
    } catch (error) {
      showError(error);
    }
  };

  const remove = async (id: string) => {
    try {
      await projectDelete(id);
      setDeleteId(null);
      void queryClient.invalidateQueries();
    } catch (error) {
      showError(error);
    }
  };

  return (
    <Dialog
      open={open}
      title={t("projects:title")}
      onClose={onClose}
      footer={
        <>
          <Button onClick={onClose}>{t("common:actions.cancel")}</Button>
          <Button variant="primary" onClick={() => void create()} disabled={!name.trim()}>
            {t("common:actions.create")}
          </Button>
        </>
      }
    >
      <div className="space-y-2">
        <label className="block">
          <span className="text-[12px] text-[var(--text-muted)]">{t("projects:name")}</span>
          <Input value={name} onChange={(event) => setName(event.target.value)} autoFocus />
        </label>
        <label className="block">
          <span className="text-[12px] text-[var(--text-muted)]">{t("projects:description")}</span>
          <Textarea
            rows={2}
            value={description}
            onChange={(event) => setDescription(event.target.value)}
          />
        </label>
        <label className="block">
          <span className="text-[12px] text-[var(--text-muted)]">{t("projects:color")}</span>
          <input
            type="color"
            value={color}
            onChange={(event) => setColor(event.target.value)}
            className="h-8 w-20 rounded border border-[var(--border-strong)] bg-transparent p-0.5"
          />
        </label>
      </div>

      <div className="mt-4 border-t border-[var(--border)] pt-3">
        <h3 className="mb-1 text-[12px] font-semibold uppercase tracking-wide text-[var(--text-muted)]">
          {t("projects:title")}
        </h3>
        <ul className="space-y-1">
          {(projects.data ?? []).map((project) => (
            <li key={project.id} className="flex items-center gap-2 text-[12.5px]">
              <span className="h-2 w-2 rounded-full" style={{ backgroundColor: project.color }} />
              <span className="min-w-0 flex-1 truncate">{project.name}</span>
              <span className="text-[11px] text-[var(--text-muted)]">
                {t("projects:count", { count: project.noteCount })}
              </span>
              {deleteId === project.id ? (
                <>
                  <Button size="sm" variant="danger" onClick={() => void remove(project.id)}>
                    {t("projects:deleteConfirm")}
                  </Button>
                  <Button size="sm" variant="ghost" onClick={() => setDeleteId(null)}>
                    {t("common:actions.cancel")}
                  </Button>
                </>
              ) : (
                <Button
                  size="icon"
                  variant="ghost"
                  className="h-6 w-6 text-[var(--danger)]"
                  aria-label={t("projects:deleteTitle")}
                  title={t("projects:deleteTitle")}
                  onClick={() => setDeleteId(project.id)}
                >
                  <Trash2 size={12} />
                </Button>
              )}
            </li>
          ))}
        </ul>
        {(projects.data ?? []).length === 0 ? (
          <p className="mt-1 text-[12px] text-[var(--text-muted)]">{t("projects:empty.title")}</p>
        ) : null}
        <p className="mt-2 text-[10.5px] text-[var(--text-muted)]">
          <Plus size={10} className="mr-1 inline" />
          {t("projects:deleteMessage", { name: "" })}
        </p>
      </div>
    </Dialog>
  );
}
