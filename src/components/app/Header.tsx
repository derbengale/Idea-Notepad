import { useUi } from "@/app/store";
import { Button } from "@/components/ui/button";
import { AccountMenu } from "@/features/account/AccountMenu";
import type { EntryListItem, ProjectDto, SchemaDto } from "@/lib/ipc/types";
import { ArrowLeft, ArrowRight, Command, HelpCircle, Search, Settings } from "lucide-react";
import { useEffect, useRef, useState, type RefObject } from "react";
import { useTranslation } from "react-i18next";

export interface HeaderProps {
  selected: EntryListItem | undefined;
  schemas: SchemaDto[];
  projects: ProjectDto[];
  searchRef: RefObject<HTMLInputElement | null>;
}

export function Header({ selected, schemas, projects, searchRef }: HeaderProps) {
  const { t } = useTranslation(["common", "entries"]);
  const {
    search,
    setSearch,
    navigateEntry,
    back,
    forward,
    historyIndex,
    history,
    setPaletteOpen,
    openSettings,
    setShortcutsOpen,
  } = useUi();
  const [value, setValue] = useState(search);
  const timer = useRef<number | undefined>(undefined);

  useEffect(() => {
    setValue(search);
  }, [search]);

  useEffect(() => {
    if (value === search) {
      return;
    }
    timer.current = window.setTimeout(() => setSearch(value), 150);
    return () => {
      if (timer.current) {
        window.clearTimeout(timer.current);
      }
    };
  }, [value, search, setSearch]);

  const schema = selected ? schemas.find((item) => item.id === selected.schemaId) : undefined;
  const project = selected ? projects.find((item) => item.id === selected.projectId) : undefined;

  return (
    <header className="flex items-center gap-2 border-b border-[var(--border)] bg-[var(--bg-elevated)] px-3 py-2">
      <span className="mr-1 select-none text-[13px] font-semibold tracking-tight">
        {t("common:appName")}
      </span>
      <Button
        variant="ghost"
        size="icon"
        className="h-7 w-7"
        aria-label={t("common:actions.back")}
        title={t("common:actions.back")}
        disabled={historyIndex <= 0}
        onClick={back}
      >
        <ArrowLeft size={14} />
      </Button>
      <Button
        variant="ghost"
        size="icon"
        className="h-7 w-7"
        aria-label={t("common:actions.forward")}
        title={t("common:actions.forward")}
        disabled={historyIndex >= history.length - 1}
        onClick={forward}
      >
        <ArrowRight size={14} />
      </Button>

      <div className="flex min-w-0 flex-1 items-center gap-1.5 px-1 text-[12px] text-[var(--text-muted)]">
        {selected ? (
          <>
            <span className="shrink-0">{schema?.name ?? selected.schemaKey}</span>
            <span>/</span>
            {project ? (
              <>
                <span className="shrink-0">{project.name}</span>
                <span>/</span>
              </>
            ) : null}
            <button
              type="button"
              className="truncate text-[var(--text)] hover:underline"
              onClick={() => navigateEntry(selected.id)}
              title={selected.title}
            >
              {selected.title}
            </button>
          </>
        ) : null}
      </div>

      <div className="relative hidden w-[260px] shrink-0 sm:block">
        <Search
          size={13}
          className="pointer-events-none absolute left-2 top-1/2 -translate-y-1/2 text-[var(--text-muted)]"
        />
        <input
          ref={searchRef}
          value={value}
          onChange={(event) => setValue(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === "Escape") {
              setValue("");
              setSearch("");
              event.currentTarget.blur();
            }
          }}
          placeholder={t("common:labels.searchPlaceholder")}
          aria-label={t("common:actions.search")}
          className="h-7 w-full rounded-[var(--r-sm)] border border-[var(--border)] bg-[var(--bg-subtle)] pl-7 pr-2 text-[12px] outline-none placeholder:text-[var(--text-muted)] focus:border-[var(--accent)]"
        />
      </div>

      <Button
        variant="ghost"
        size="icon"
        className="h-7 w-7"
        title={t("common:palette.placeholder")}
        aria-label={t("common:palette.placeholder")}
        onClick={() => setPaletteOpen(true)}
      >
        <Command size={14} />
      </Button>
      <Button
        variant="ghost"
        size="icon"
        className="h-7 w-7"
        title={t("common:shortcuts.open")}
        aria-label={t("common:shortcuts.open")}
        onClick={() => setShortcutsOpen(true)}
      >
        <HelpCircle size={14} />
      </Button>
      <AccountMenu />
      <Button
        variant="ghost"
        size="icon"
        className="h-7 w-7"
        title={t("common:actions.settings")}
        aria-label={t("common:actions.settings")}
        onClick={() => openSettings("general")}
      >
        <Settings size={14} />
      </Button>
    </header>
  );
}
