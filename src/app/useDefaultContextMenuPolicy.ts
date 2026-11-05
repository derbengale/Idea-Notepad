import { useEffect } from "react";

const TEXT_CONTEXT_SELECTOR = "input, textarea, [contenteditable='true'], .cm-editor";

/**
 * Keeps the WebView2 default context menu only where it is useful
 * (text fields and active text selections) and suppresses it everywhere else,
 * so that app-specific context menus are the only visible ones.
 */
export function useDefaultContextMenuPolicy() {
  useEffect(() => {
    const onContextMenu = (event: MouseEvent) => {
      const target = event.target;
      const element = target instanceof Element ? target : null;
      const selection = window.getSelection();
      const hasSelection = selection !== null && selection.toString().trim().length > 0;
      if (hasSelection || (element !== null && element.closest(TEXT_CONTEXT_SELECTOR) !== null)) {
        return;
      }
      event.preventDefault();
    };
    window.addEventListener("contextmenu", onContextMenu);
    return () => window.removeEventListener("contextmenu", onContextMenu);
  }, []);
}
