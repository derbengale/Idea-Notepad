import { useIpcError } from "@/app/errors";
import { Button } from "@/components/ui/button";
import { Dialog } from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { linkList } from "@/lib/ipc";
import type { EntryListItem } from "@/lib/ipc/types";
import { formatRelative } from "@/lib/utils/format";
import { useQuery } from "@tanstack/react-query";
import { Link2, Loader2 } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

export interface LinkDialogProps {
  open: boolean;
  source: EntryListItem | null;
  entries: EntryListItem[];
  onClose: () => void;
  onLink: (targetId: string) => void;
}

export function LinkDialog({ open, source, entries, onClose, onLink }: LinkDialogProps) {
  const { t, i18n } = useTranslation(["entries", "common"]);
  const showError = useIpcError();
  const [search, setSearch] = useState("");

  const linkedQuery = useQuery({
    queryKey: ["links", source?.id ?? ""],
    queryFn: () => linkList(source?.id ?? ""),
    enabled: open && source !== null,
  });

  useEffect(() => {
    if (!open) {
      setSearch("");
    }
  }, [open]);

  useEffect(() => {
    if (linkedQuery.isError) {
      showError(linkedQuery.error);
    }
  }, [linkedQuery.isError, linkedQuery.error, showError]);

  const linkedIds = useMemo(
    () => new Set((linkedQuery.data?.links ?? []).map((link) => link.entryId)),
    [linkedQuery.data],
  );

  const candidates = useMemo(() => {
    const term = search.trim().toLowerCase();
    return entries.filter((item) => {
      if (!source || item.id === source.id || item.deletedAt !== null) {
        return false;
      }
      if (linkedIds.has(item.id)) {
        return false;
      }
      if (!term) {
        return true;
      }
      return (
        item.title.toLowerCase().includes(term) ||
        (item.subtitle ?? "").toLowerCase().includes(term)
      );
    });
  }, [entries, linkedIds, search, source]);

  return (
    <Dialog
      open={open}
      title={t("entries:detail.addLink")}
      onClose={onClose}
      width="min(520px, 92vw)"
      footer={<Button onClick={onClose}>{t("common:actions.cancel")}</Button>}
    >
      <div className="flex flex-col gap-2">
        <Input
          value={search}
          onChange={(event) => setSearch(event.target.value)}
          placeholder={t("entries:linkDialog.searchPlaceholder")}
          aria-label={t("entries:detail.linkPlaceholder")}
        />
        {linkedQuery.isLoading ? (
          <div className="flex items-center justify-center py-8 text-[var(--text-muted)]">
            <Loader2 size={16} className="animate-spin" />
          </div>
        ) : candidates.length === 0 ? (
          <p className="py-6 text-center text-[12px] text-[var(--text-muted)]">
            {search.trim()
              ? t("entries:linkDialog.noResults")
              : t("entries:linkDialog.empty")}
          </p>
        ) : (
          <ul className="max-h-[46vh] overflow-auto rounded-[var(--r-sm)] border border-[var(--border)]">
            {candidates.slice(0, 200).map((item) => (
              <li key={item.id} className="border-b border-[var(--border)] last:border-b-0">
                <button
                  type="button"
                  onClick={() => onLink(item.id)}
                  className="flex w-full items-center gap-2 px-2.5 py-2 text-left hover:bg-[var(--bg-subtle)]"
                >
                  <Link2 size={13} className="shrink-0 text-[var(--text-muted)]" />
                  <span className="min-w-0 flex-1">
                    <span className="block truncate text-[13px]">{item.title}</span>
                    <span className="block truncate text-[11px] text-[var(--text-muted)]">
                      {item.subtitle ?? item.schemaKey}
                    </span>
                  </span>
                  <span className="shrink-0 text-[11px] text-[var(--text-muted)]">
                    {formatRelative(item.updatedAt, i18n.language)}
                  </span>
                </button>
              </li>
            ))}
          </ul>
        )}
      </div>
    </Dialog>
  );
}
