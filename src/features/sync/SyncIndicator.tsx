import { useIpcError } from "@/app/errors";
import { Button } from "@/components/ui/button";
import { syncNow, syncPause, syncResume, syncStatus } from "@/lib/ipc";
import { formatRelative } from "@/lib/utils/format";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { Cloud, CloudOff, Loader2, Pause, Play, RefreshCw, TriangleAlert } from "lucide-react";
import { useTranslation } from "react-i18next";

export function SyncIndicator() {
  const { t, i18n } = useTranslation(["sync", "common"]);
  const queryClient = useQueryClient();
  const showError = useIpcError();
  const status = useQuery({
    queryKey: ["sync"],
    queryFn: syncStatus,
    refetchInterval: 10_000,
  });
  const state = status.data?.state ?? "idle";

  const run = async (action: "now" | "pause" | "resume") => {
    try {
      if (action === "now") {
        await syncNow();
      } else if (action === "pause") {
        await syncPause();
      } else {
        await syncResume();
      }
      void queryClient.invalidateQueries({ queryKey: ["sync"] });
      void queryClient.invalidateQueries({ queryKey: ["entries"] });
    } catch (error) {
      showError(error);
    }
  };

  const icon =
    state === "syncing" ? (
      <Loader2 size={13} className="animate-spin" />
    ) : state === "offline" ? (
      <CloudOff size={13} />
    ) : state === "paused" ? (
      <Pause size={13} />
    ) : state === "error" ? (
      <TriangleAlert size={13} />
    ) : (
      <Cloud size={13} />
    );

  return (
    <div className="space-y-1">
      <div
        className="flex items-center gap-1.5 text-[11.5px] text-[var(--text-muted)]"
        title={status.data?.lastSyncAt ?? t("sync:never")}
      >
        {icon}
        <span>{t(`sync:state.${state}`)}</span>
        {status.data && status.data.pendingOperations > 0 ? (
          <span className="text-[10.5px]">({status.data.pendingOperations})</span>
        ) : null}
      </div>
      {status.data?.lastSyncAt ? (
        <div className="text-[10px] text-[var(--text-muted)]">
          {t("sync:lastSync", {
            when: formatRelative(status.data.lastSyncAt, i18n.language),
          })}
        </div>
      ) : null}
      <div className="flex gap-1">
        <Button
          size="sm"
          variant="ghost"
          className="h-6 px-1.5 text-[11px]"
          onClick={() => void run("now")}
          disabled={state === "syncing"}
        >
          <RefreshCw size={11} />
          {t("sync:syncNow")}
        </Button>
        {state === "paused" ? (
          <Button
            size="sm"
            variant="ghost"
            className="h-6 px-1.5 text-[11px]"
            onClick={() => void run("resume")}
          >
            <Play size={11} />
            {t("common:actions.resume")}
          </Button>
        ) : (
          <Button
            size="sm"
            variant="ghost"
            className="h-6 px-1.5 text-[11px]"
            onClick={() => void run("pause")}
          >
            <Pause size={11} />
            {t("common:actions.pause")}
          </Button>
        )}
      </div>
    </div>
  );
}
