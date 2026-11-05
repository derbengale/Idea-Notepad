import { useToast } from "@/components/ui/toast";
import { IpcError } from "@/lib/ipc";
import { useCallback } from "react";
import { useTranslation } from "react-i18next";

export function useIpcError() {
  const { push } = useToast();
  const { t } = useTranslation("errors");

  return useCallback(
    (error: unknown) => {
      if (error instanceof IpcError) {
        const title = t(`${error.code}.title`, { defaultValue: t("UNKNOWN.title") });
        const description = t(`${error.code}.description`, {
          defaultValue: t("UNKNOWN.description"),
        });
        push(`${title}: ${description}`, { tone: "error", duration: 0 });
        return;
      }
      push(error instanceof Error ? error.message : String(error), {
        tone: "error",
        duration: 0,
      });
    },
    [push, t],
  );
}
