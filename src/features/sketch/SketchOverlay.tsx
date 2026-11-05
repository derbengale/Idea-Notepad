import { useIpcError } from "@/app/errors";
import { Button } from "@/components/ui/button";
import { sketchLoad, sketchSave } from "@/lib/ipc";
import { X } from "lucide-react";
import { lazy, Suspense, useEffect, useMemo, useRef, useState, type ComponentType } from "react";
import { useTranslation } from "react-i18next";

interface ExcalidrawPropsLoose {
  initialData?: Record<string, unknown>;
  onChange?: (elements: readonly unknown[], appState: unknown, files: unknown) => void;
  theme?: "light" | "dark";
}

const ExcalidrawCanvas = lazy(async () => {
  const module = await import("@excalidraw/excalidraw");
  return {
    default: module.Excalidraw as unknown as ComponentType<ExcalidrawPropsLoose>,
  };
});

export interface SketchOverlayProps {
  sketchId: string;
  onClose: () => void;
}

interface SceneState {
  elements: readonly unknown[];
  appState: unknown;
  files: unknown;
}

export function SketchOverlay({ sketchId, onClose }: SketchOverlayProps) {
  const { t } = useTranslation("editor");
  const showError = useIpcError();
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState(false);
  const [initialData, setInitialData] = useState<Record<string, unknown> | null>(null);
  const scene = useRef<SceneState>({ elements: [], appState: {}, files: {} });
  const theme = document.documentElement.dataset.theme === "dark" ? "dark" : "light";

  useEffect(() => {
    let cancelled = false;
    void (async () => {
      try {
        const data = await sketchLoad(sketchId);
        if (cancelled) {
          return;
        }
        const parsed: Record<string, unknown> = JSON.parse(data.sceneJson) as Record<
          string,
          unknown
        >;
        scene.current = {
          elements: Array.isArray(parsed.elements) ? (parsed.elements as unknown[]) : [],
          appState: parsed.appState ?? {},
          files: parsed.files ?? {},
        };
        setInitialData({
          elements: scene.current.elements,
          appState: scene.current.appState,
          files: scene.current.files,
        });
      } catch {
        setError(true);
      } finally {
        if (!cancelled) {
          setLoading(false);
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [sketchId]);

  const handleClose = async () => {
    if (saving) {
      return;
    }
    setSaving(true);
    try {
      let preview: string | null = null;
      if (scene.current.elements.length > 0) {
        const module = await import("@excalidraw/excalidraw");
        const exportToBlob = module.exportToBlob as unknown as (options: {
          elements: readonly unknown[];
          appState: unknown;
          files: unknown;
          mimeType: string;
        }) => Promise<Blob>;
        const blob = await exportToBlob({
          elements: scene.current.elements,
          appState: scene.current.appState,
          files: serializeFiles(scene.current.files),
          mimeType: "image/png",
        });
        preview = await blobToBase64(blob);
      }
      const payload = {
        type: "excalidraw",
        version: 2,
        source: "better-notepad",
        elements: scene.current.elements,
        appState: pickAppState(scene.current.appState),
        files: serializeFiles(scene.current.files),
      };
      await sketchSave(sketchId, JSON.stringify(payload), preview);
      onClose();
    } catch (caught) {
      showError(caught);
    } finally {
      setSaving(false);
    }
  };

  const initial = useMemo(() => initialData, [initialData]);

  return (
    <div className="fixed inset-0 z-50 flex flex-col bg-[var(--bg)]">
      <header className="flex items-center justify-between border-b border-[var(--border)] bg-[var(--bg-elevated)] px-4 py-2">
        <h2 className="text-[14px] font-semibold">{t("sketch.title")}</h2>
        <div className="flex items-center gap-2">
          <span className="text-[12px] text-[var(--text-muted)]">
            {saving ? t("sketch.saving") : ""}
          </span>
          <Button
            variant="ghost"
            size="icon"
            onClick={() => void handleClose()}
            aria-label={t("mode.source")}
          >
            <X size={16} />
          </Button>
        </div>
      </header>
      <div className="min-h-0 flex-1">
        {error ? (
          <div className="flex h-full items-center justify-center text-[13px] text-[var(--danger)]">
            {t("sketch.error")}
          </div>
        ) : loading || !initial ? (
          <div className="flex h-full items-center justify-center text-[13px] text-[var(--text-muted)]">
            {t("sketch.saving")}
          </div>
        ) : (
          <Suspense
            fallback={
              <div className="flex h-full items-center justify-center text-[13px] text-[var(--text-muted)]">
                {t("sketch.saving")}
              </div>
            }
          >
            <ExcalidrawCanvas
              theme={theme}
              initialData={initial}
              onChange={(elements, appState, files) => {
                scene.current = { elements, appState, files };
              }}
            />
          </Suspense>
        )}
      </div>
    </div>
  );
}

function serializeFiles(files: unknown): unknown {
  if (files instanceof Map) {
    return Object.fromEntries(files as Map<string, unknown>);
  }
  return files ?? {};
}

function pickAppState(appState: unknown): Record<string, unknown> {
  if (typeof appState !== "object" || appState === null) {
    return {};
  }
  const source = appState as Record<string, unknown>;
  return {
    viewBackgroundColor: source.viewBackgroundColor ?? "#ffffff",
    gridSize: source.gridSize ?? null,
  };
}

async function blobToBase64(blob: Blob): Promise<string> {
  const buffer = await blob.arrayBuffer();
  let binary = "";
  const bytes = new Uint8Array(buffer);
  const chunk = 0x8000;
  for (let index = 0; index < bytes.length; index += chunk) {
    binary += String.fromCharCode(...bytes.subarray(index, index + chunk));
  }
  return btoa(binary);
}
