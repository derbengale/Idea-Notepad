import { cn } from "@/lib/utils/cn";
import { Check, ChevronRight } from "lucide-react";
import {
    useCallback,
    useEffect,
    useLayoutEffect,
    useMemo,
    useRef,
    useState,
    type CSSProperties,
    type KeyboardEvent as ReactKeyboardEvent,
    type ReactNode,
} from "react";
import { createPortal } from "react-dom";

export interface ContextMenuItem {
  id: string;
  label: string;
  icon?: ReactNode;
  shortcut?: string;
  tone?: "default" | "danger";
  disabled?: boolean;
  checked?: boolean;
  submenu?: ContextMenuItem[];
  onSelect?: () => void;
}

export interface ContextMenuSeparator {
  id: string;
  separator: true;
}

export type ContextMenuEntry = ContextMenuItem | ContextMenuSeparator;

function isSeparator(entry: ContextMenuEntry | undefined): entry is ContextMenuSeparator {
  return entry !== undefined && "separator" in entry && entry.separator;
}

export interface ContextMenuPoint {
  x: number;
  y: number;
}

export interface ContextMenuEventLike {
  clientX: number;
  clientY: number;
  preventDefault: () => void;
  stopPropagation: () => void;
}

export interface ContextMenuState {
  point: ContextMenuPoint;
  entries: ContextMenuEntry[];
}

export function useContextMenu() {
  const [state, setState] = useState<ContextMenuState | null>(null);

  const openAt = useCallback((event: ContextMenuEventLike, entries: ContextMenuEntry[]) => {
    event.preventDefault();
    event.stopPropagation();
    setState({ point: { x: event.clientX, y: event.clientY }, entries });
  }, []);

  const close = useCallback(() => setState(null), []);

  return { state, openAt, close };
}

type MenuAnchor =
  | { kind: "point"; x: number; y: number }
  | { kind: "element"; rect: DOMRect };

interface MenuPanelProps {
  entries: ContextMenuEntry[];
  anchor: MenuAnchor;
  label: string;
  level: number;
  onActivate: () => void;
  onDismiss: () => void;
}

const VIEWPORT_MARGIN = 8;

function MenuPanel({ entries, anchor, label, level, onActivate, onDismiss }: MenuPanelProps) {
  const panelRef = useRef<HTMLDivElement>(null);
  const itemRefs = useRef(new Map<string, HTMLButtonElement>());
  const [position, setPosition] = useState<CSSProperties>({
    left: -9999,
    top: -9999,
    visibility: "hidden",
  });
  const [activeIndex, setActiveIndex] = useState(-1);
  const [submenu, setSubmenu] = useState<{ id: string; rect: DOMRect } | null>(null);

  useLayoutEffect(() => {
    const panel = panelRef.current;
    if (!panel) {
      return;
    }
    const { width, height } = panel.getBoundingClientRect();
    let left = anchor.kind === "point" ? anchor.x : anchor.rect.right + 2;
    let top = anchor.kind === "point" ? anchor.y : anchor.rect.top;
    if (left + width > window.innerWidth - VIEWPORT_MARGIN) {
      left =
        anchor.kind === "point"
          ? Math.max(VIEWPORT_MARGIN, window.innerWidth - VIEWPORT_MARGIN - width)
          : Math.max(VIEWPORT_MARGIN, anchor.rect.left - width - 2);
    }
    if (top + height > window.innerHeight - VIEWPORT_MARGIN) {
      top = Math.max(VIEWPORT_MARGIN, window.innerHeight - VIEWPORT_MARGIN - height);
    }
    setPosition((current) =>
      current.left === left && current.top === top ? current : { left, top, visibility: "visible" },
    );
  }, [anchor]);

  useEffect(() => {
    panelRef.current?.focus();
  }, []);

  useEffect(() => {
    if (level !== 1) {
      return;
    }
    const onPointerDown = (event: PointerEvent) => {
      if (panelRef.current?.contains(event.target as Node)) {
        return;
      }
      onDismiss();
    };
    const onWindowBlur = () => onDismiss();
    const onScroll = () => onDismiss();
    document.addEventListener("pointerdown", onPointerDown, true);
    document.addEventListener("scroll", onScroll, true);
    window.addEventListener("blur", onWindowBlur);
    window.addEventListener("resize", onWindowBlur);
    return () => {
      document.removeEventListener("pointerdown", onPointerDown, true);
      document.removeEventListener("scroll", onScroll, true);
      window.removeEventListener("blur", onWindowBlur);
      window.removeEventListener("resize", onWindowBlur);
    };
  }, [level, onDismiss]);

  const openSubmenu = (item: ContextMenuItem) => {
    const element = itemRefs.current.get(item.id);
    if (!element) {
      return;
    }
    setSubmenu({ id: item.id, rect: element.getBoundingClientRect() });
  };

  const activate = (item: ContextMenuItem) => {
    if (item.disabled) {
      return;
    }
    if (item.submenu && item.submenu.length > 0) {
      openSubmenu(item);
      return;
    }
    onActivate();
    onDismiss();
    item.onSelect?.();
  };

  const findEnabledIndex = (from: number, step: 1 | -1): number => {
    let index = from;
    for (let guard = 0; guard < entries.length; guard += 1) {
      index = (index + step + entries.length) % entries.length;
      const candidate = entries[index];
      if (candidate && !isSeparator(candidate) && !candidate.disabled) {
        return index;
      }
    }
    return -1;
  };

  const onKeyDown = (event: ReactKeyboardEvent<HTMLDivElement>) => {
    if (event.key === "Escape") {
      event.preventDefault();
      event.stopPropagation();
      if (submenu) {
        setSubmenu(null);
      } else {
        onDismiss();
      }
      return;
    }
    if (event.key === "Tab") {
      onDismiss();
      return;
    }
    if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault();
      setActiveIndex((current) => findEnabledIndex(current, event.key === "ArrowDown" ? 1 : -1));
      return;
    }
    if (event.key === "Home" || event.key === "End") {
      event.preventDefault();
      setActiveIndex(findEnabledIndex(event.key === "Home" ? -1 : 0, event.key === "Home" ? 1 : -1));
      return;
    }
    if (event.key === "ArrowRight") {
      const current = entries[activeIndex];
      if (current && !isSeparator(current) && current.submenu) {
        event.preventDefault();
        openSubmenu(current);
      }
      return;
    }
    if (event.key === "ArrowLeft" && level > 1) {
      event.preventDefault();
      onDismiss();
      return;
    }
    if (event.key === "Enter" || event.key === " ") {
      const current = entries[activeIndex];
      if (current && !isSeparator(current)) {
        event.preventDefault();
        activate(current);
      }
    }
  };

  const submenuEntries = useMemo(() => {
    if (!submenu) {
      return [];
    }
    for (const entry of entries) {
      if (!isSeparator(entry) && entry.id === submenu.id) {
        return entry.submenu ?? [];
      }
    }
    return [];
  }, [entries, submenu]);

  const panel = (
    <div
      ref={panelRef}
      role="menu"
      aria-label={label}
      aria-orientation="vertical"
      tabIndex={-1}
      style={position}
      onKeyDown={onKeyDown}
      onContextMenu={(event) => event.preventDefault()}
      className="fixed z-[60] min-w-[224px] select-none rounded-[var(--r-md)] border border-[var(--border)] bg-[var(--bg-elevated)] p-1 shadow-[var(--shadow-md)] outline-none"
    >
      {entries.map((entry, index) => {
        if (isSeparator(entry)) {
          return (
            <div key={entry.id} role="separator" className="my-1 h-px bg-[var(--border)]" />
          );
        }
        const active = index === activeIndex;
        return (
          <button
            key={entry.id}
            ref={(element) => {
              if (element) {
                itemRefs.current.set(entry.id, element);
              } else {
                itemRefs.current.delete(entry.id);
              }
            }}
            type="button"
            role="menuitem"
            aria-haspopup={entry.submenu ? "menu" : undefined}
            aria-expanded={entry.submenu ? submenu?.id === entry.id : undefined}
            aria-disabled={entry.disabled || undefined}
            disabled={entry.disabled}
            tabIndex={-1}
            onMouseEnter={() => {
              setActiveIndex(index);
              if (entry.submenu) {
                openSubmenu(entry);
              } else {
                setSubmenu(null);
              }
            }}
            onClick={() => activate(entry)}
            className={cn(
              "flex w-full items-center gap-2 rounded-[var(--r-sm)] px-2 py-1.5 text-left text-[12.5px] transition-colors",
              entry.tone === "danger" ? "text-[var(--danger)]" : "text-[var(--text)]",
              active ? "bg-[var(--bg-subtle)]" : "",
              entry.disabled ? "cursor-default opacity-45" : "cursor-default",
            )}
          >
            <span className="flex w-4 shrink-0 items-center justify-center text-[var(--text-muted)]">
              {entry.icon}
            </span>
            <span className="min-w-0 flex-1 truncate">{entry.label}</span>
            {entry.checked ? <Check size={12} className="shrink-0 text-[var(--accent)]" /> : null}
            {entry.shortcut ? (
              <kbd className="shrink-0 font-sans text-[10px] uppercase tracking-wide text-[var(--text-muted)]">
                {entry.shortcut}
              </kbd>
            ) : null}
            {entry.submenu ? (
              <ChevronRight size={12} className="shrink-0 text-[var(--text-muted)]" />
            ) : null}
          </button>
        );
      })}
      {submenu && submenuEntries.length > 0 ? (
        <MenuPanel
          entries={submenuEntries}
          anchor={{ kind: "element", rect: submenu.rect }}
          label={label}
          level={level + 1}
          onActivate={onActivate}
          onDismiss={() => {
            setSubmenu(null);
            panelRef.current?.focus();
          }}
        />
      ) : null}
    </div>
  );

  return level === 1 ? createPortal(panel, document.body) : panel;
}

export interface ContextMenuProps {
  point: ContextMenuPoint;
  entries: ContextMenuEntry[];
  label: string;
  onClose: () => void;
}

export function ContextMenu({ point, entries, label, onClose }: ContextMenuProps) {
  const activated = useRef(false);
  const previousFocus = useRef<HTMLElement | null>(
    document.activeElement instanceof HTMLElement ? document.activeElement : null,
  );

  useEffect(
    () => () => {
      if (!activated.current) {
        previousFocus.current?.focus();
      }
    },
    [],
  );

  const anchor = useMemo<MenuAnchor>(() => ({ kind: "point", x: point.x, y: point.y }), [point.x, point.y]);

  return (
    <MenuPanel
      entries={entries}
      anchor={anchor}
      label={label}
      level={1}
      onActivate={() => {
        activated.current = true;
      }}
      onDismiss={onClose}
    />
  );
}
