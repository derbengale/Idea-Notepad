import { useUi } from "@/app/store";
import { Button } from "@/components/ui/button";
import { Dialog } from "@/components/ui/dialog";
import { useTranslation } from "react-i18next";

const shortcuts: Array<[string, string]> = [
  ["Ctrl+N", "newNote"],
  ["Ctrl+Shift+N", "newNoteWithSchema"],
  ["Ctrl+Shift+S", "newSchema"],
  ["Ctrl+S", "save"],
  ["Ctrl+Z / Ctrl+Shift+Z", "undo"],
  ["Ctrl+F", "search"],
  ["Ctrl+K / Ctrl+P", "palette"],
  ["Ctrl+E", "editorMode"],
  ["Ctrl+B", "sidebar"],
  ["Ctrl+1…9", "schemas"],
  ["Alt+← / Alt+→", "back"],
  ["Delete", "deleteNote"],
  ["Esc", "escape"],
  ["Ctrl+/", "help"],
];

export function ShortcutsSheet() {
  const { t } = useTranslation(["shortcuts", "common"]);
  const { shortcutsOpen, setShortcutsOpen } = useUi();
  return (
    <Dialog
      open={shortcutsOpen}
      title={t("shortcuts:title")}
      onClose={() => setShortcutsOpen(false)}
      footer={
        <Button variant="primary" onClick={() => setShortcutsOpen(false)}>
          {t("common:actions.close")}
        </Button>
      }
    >
      <table className="w-full text-[12.5px]">
        <tbody>
          {shortcuts.map(([keys, label]) => (
            <tr key={keys} className="border-b border-[var(--border)] last:border-0">
              <td className="w-[180px] py-1.5">
                <kbd className="rounded bg-[var(--bg-subtle)] px-1.5 py-0.5 font-mono text-[11px]">
                  {keys}
                </kbd>
              </td>
              <td className="py-1.5 text-[var(--text-muted)]">{t(`shortcuts:${label}`)}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </Dialog>
  );
}
