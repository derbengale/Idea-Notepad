import { Button } from "@/components/ui/button";
import { ContextMenu, useContextMenu, type ContextMenuEntry } from "@/components/ui/context-menu";
import type { AttachmentDto } from "@/lib/ipc/types";
import { cn } from "@/lib/utils/cn";
import { convertFileSrc } from "@tauri-apps/api/core";
import { ImagePlus, Pencil, Trash2 } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

export interface SketchGalleryProps {
  sketches: AttachmentDto[];
  readOnly?: boolean;
  onNew: () => void;
  onOpen: (id: string) => void;
  onDelete: (id: string) => void;
}

export function SketchGallery({
  sketches,
  readOnly = false,
  onNew,
  onOpen,
  onDelete,
}: SketchGalleryProps) {
  const { t } = useTranslation("editor");
  const { state: contextMenu, openAt, close: closeContextMenu } = useContextMenu();
  if (sketches.length === 0) {
    return (
      <div className="flex items-center gap-2">
        {readOnly ? null : (
          <Button size="sm" onClick={onNew}>
            <ImagePlus size={14} />
            {t("sketch.new")}
          </Button>
        )}
        <span className="text-[12px] text-[var(--text-muted)]">{t("sketch.empty")}</span>
      </div>
    );
  }
  return (
    <div className="flex flex-wrap gap-3">
      {sketches.map((sketch) => {
        const entries: ContextMenuEntry[] = [
          {
            id: "open",
            label: t("sketch.open"),
            icon: <Pencil size={13} />,
            onSelect: () => onOpen(sketch.id),
          },
        ];
        if (!readOnly) {
          entries.push({
            id: "delete",
            label: t("sketch.delete"),
            icon: <Trash2 size={13} />,
            tone: "danger",
            onSelect: () => onDelete(sketch.id),
          });
        }
        return (
        <div
          key={sketch.id}
          className="group relative w-40 overflow-hidden rounded-[var(--r-md)] border border-[var(--border)] bg-[var(--bg-subtle)]"
          onContextMenu={(event) => openAt(event, entries)}
        >
          <button
            type="button"
            className="block h-24 w-full overflow-hidden"
            onClick={() => onOpen(sketch.id)}
            title={t("sketch.open")}
          >
            <SketchThumbnail sketch={sketch} />
          </button>
          <div className="flex items-center justify-between border-t border-[var(--border)] px-2 py-1">
            <span className="truncate text-[11px] text-[var(--text-muted)]">
              {t("sketch.title")}
            </span>
            <span className="flex gap-0.5 opacity-70 transition-opacity group-hover:opacity-100">
              <Button
                size="icon"
                variant="ghost"
                className="h-6 w-6"
                title={t("sketch.open")}
                aria-label={t("sketch.open")}
                onClick={() => onOpen(sketch.id)}
              >
                <Pencil size={12} />
              </Button>
              {readOnly ? null : (
                <Button
                  size="icon"
                  variant="ghost"
                  className="h-6 w-6 text-[var(--danger)]"
                  title={t("sketch.delete")}
                  aria-label={t("sketch.delete")}
                  onClick={() => onDelete(sketch.id)}
                >
                  <Trash2 size={12} />
                </Button>
              )}
            </span>
          </div>
        </div>
        );
      })}
      {readOnly ? null : (
        <button
          type="button"
          onClick={onNew}
          className={cn(
            "flex h-[122px] w-40 flex-col items-center justify-center gap-1 rounded-[var(--r-md)] border border-dashed border-[var(--border-strong)] text-[12px] text-[var(--text-muted)] hover:bg-[var(--bg-subtle)]",
          )}
        >
          <ImagePlus size={18} />
          {t("sketch.new")}
        </button>
      )}

      {contextMenu ? (
        <ContextMenu
          point={contextMenu.point}
          entries={contextMenu.entries}
          label={t("sketch.title")}
          onClose={closeContextMenu}
        />
      ) : null}
    </div>
  );
}

function SketchThumbnail({ sketch }: { sketch: AttachmentDto }) {
  const [failed, setFailed] = useState(false);
  const [source, setSource] = useState<string | null>(null);

  useEffect(() => {
    if (!sketch.previewPath) {
      setFailed(true);
      return;
    }
    setSource(convertFileSrc(sketch.previewPath));
  }, [sketch.previewPath]);

  if (!source || failed) {
    return (
      <div className="flex h-full items-center justify-center text-[11px] text-[var(--text-muted)]">
        {sketch.fileName}
      </div>
    );
  }
  return (
    <img
      src={source}
      alt={sketch.fileName}
      className="h-full w-full object-cover"
      onError={() => setFailed(true)}
      loading="lazy"
    />
  );
}
