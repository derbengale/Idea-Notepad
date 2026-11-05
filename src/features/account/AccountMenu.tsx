import { useIpcError } from "@/app/errors";
import { Button } from "@/components/ui/button";
import { Dialog } from "@/components/ui/dialog";
import { useToast } from "@/components/ui/toast";
import { authSignIn, authSignOut, authStatus } from "@/lib/ipc";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { LogIn, LogOut, UserRound } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";

export function AccountMenu() {
  const { t } = useTranslation(["account", "common"]);
  const queryClient = useQueryClient();
  const showError = useIpcError();
  const { push } = useToast();
  const [open, setOpen] = useState(false);
  const [signOutOpen, setSignOutOpen] = useState(false);
  const [removeLocal, setRemoveLocal] = useState(false);

  const auth = useQuery({ queryKey: ["auth"], queryFn: authStatus, refetchInterval: 30_000 });

  const signIn = useMutation({
    mutationFn: authSignIn,
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ["auth"] });
      void queryClient.invalidateQueries({ queryKey: ["sync"] });
      void queryClient.invalidateQueries({ queryKey: ["entries"] });
      push(t("account:signedInAs", { email: auth.data?.email ?? "" }), { tone: "success" });
    },
    onError: showError,
  });

  const signOut = useMutation({
    mutationFn: () => authSignOut(removeLocal),
    onSuccess: () => {
      setSignOutOpen(false);
      void queryClient.invalidateQueries();
    },
    onError: showError,
  });

  const status = auth.data;
  const label = status?.email ?? t("account:notSignedIn");

  return (
    <div className="relative">
      <Button
        variant="ghost"
        size="sm"
        onClick={() => setOpen((current) => !current)}
        aria-expanded={open}
        aria-haspopup="menu"
        title={label}
      >
        <UserRound size={14} />
        <span className="max-w-[140px] truncate text-[12px]">{label}</span>
      </Button>
      {open ? (
        <div
          role="menu"
          className="absolute right-0 top-9 z-40 w-[260px] rounded-[var(--r-md)] border border-[var(--border)] bg-[var(--bg-elevated)] p-3 shadow-[var(--shadow-md)]"
        >
          {status?.signedIn ? (
            <>
              <p className="text-[13px] font-medium">{status.name ?? status.email}</p>
              <p className="mb-2 text-[11px] text-[var(--text-muted)]">{status.email}</p>
              <Button
                variant="secondary"
                className="w-full"
                onClick={() => {
                  setOpen(false);
                  setSignOutOpen(true);
                }}
              >
                <LogOut size={13} />
                {t("common:actions.signOut")}
              </Button>
            </>
          ) : (
            <>
              <p className="mb-1 text-[12.5px]">{t("account:notSignedIn")}</p>
              <p className="mb-2 text-[11.5px] text-[var(--text-muted)]">
                {status?.hasClientId ? t("account:signInDescription") : t("account:clientIdHint")}
              </p>
              <Button
                variant="primary"
                className="w-full"
                disabled={!status?.hasClientId || signIn.isPending}
                onClick={() => signIn.mutate()}
              >
                <LogIn size={13} />
                {t("account:signIn")}
              </Button>
            </>
          )}
          <p className="mt-2 border-t border-[var(--border)] pt-2 text-[10.5px] text-[var(--text-muted)]">
            {t("account:scopeNotice")}
          </p>
        </div>
      ) : null}

      <Dialog
        open={signOutOpen}
        title={t("account:signOutTitle")}
        onClose={() => setSignOutOpen(false)}
        footer={
          <>
            <Button onClick={() => setSignOutOpen(false)}>{t("common:actions.cancel")}</Button>
            <Button variant="danger" onClick={() => signOut.mutate()} disabled={signOut.isPending}>
              {t("account:signOutConfirm")}
            </Button>
          </>
        }
      >
        <p className="mb-3 text-[13px]">{t("account:signOutMessage")}</p>
        <label className="flex items-center gap-2 text-[13px]">
          <input
            type="checkbox"
            className="h-4 w-4 accent-[var(--accent)]"
            checked={removeLocal}
            onChange={(event) => setRemoveLocal(event.target.checked)}
          />
          {t("account:removeLocal")}
        </label>
        {removeLocal ? (
          <p className="mt-1 text-[11.5px] text-[var(--danger)]">
            {t("account:removeLocalWarning")}
          </p>
        ) : null}
      </Dialog>
    </div>
  );
}
