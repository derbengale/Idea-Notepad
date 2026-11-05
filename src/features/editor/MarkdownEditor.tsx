import { Button } from "@/components/ui/button";
import { defaultKeymap, history, historyKeymap, indentWithTab } from "@codemirror/commands";
import { markdown, markdownLanguage } from "@codemirror/lang-markdown";
import { bracketMatching, defaultHighlightStyle, syntaxHighlighting } from "@codemirror/language";
import { languages } from "@codemirror/language-data";
import { Compartment, EditorState } from "@codemirror/state";
import {
  EditorView,
  drawSelection,
  highlightActiveLine,
  keymap,
  placeholder as placeholderExtension,
} from "@codemirror/view";
import {
  Bold,
  Code,
  Heading,
  Italic,
  Link2,
  List,
  ListChecks,
  Quote,
  Sigma,
  Strikethrough,
  Table,
  TextQuote,
} from "lucide-react";
import { useEffect, useRef, type ReactNode } from "react";
import { useTranslation } from "react-i18next";

export interface MarkdownEditorProps {
  value: string;
  onChange: (value: string) => void;
  onSave?: () => void;
  readOnly?: boolean;
  fontSize: number;
  spellCheck: boolean;
}

const theme = EditorView.theme({
  "&": {
    backgroundColor: "var(--bg-elevated)",
    color: "var(--text)",
    height: "100%",
    fontSize: "13px",
  },
  ".cm-content": {
    padding: "12px 16px 40px",
    fontFamily: "var(--font-code)",
    caretColor: "var(--accent)",
  },
  ".cm-gutters": {
    backgroundColor: "var(--bg-elevated)",
    color: "var(--text-muted)",
    borderRight: "1px solid var(--border)",
  },
  ".cm-activeLine": { backgroundColor: "transparent" },
  ".cm-activeLineGutter": { backgroundColor: "transparent" },
  "&.cm-focused .cm-selectionBackground, .cm-selectionBackground": {
    backgroundColor: "var(--accent-soft)",
  },
  ".cm-cursor": { borderLeftColor: "var(--accent)" },
  ".cm-tooltip": {
    backgroundColor: "var(--bg-elevated)",
    border: "1px solid var(--border)",
    color: "var(--text)",
  },
});

export function MarkdownEditor({
  value,
  onChange,
  onSave,
  readOnly = false,
  fontSize,
  spellCheck,
}: MarkdownEditorProps) {
  const { t } = useTranslation("editor");
  const host = useRef<HTMLDivElement>(null);
  const view = useRef<EditorView | null>(null);
  const onChangeRef = useRef(onChange);
  const onSaveRef = useRef(onSave);
  const readOnlyCompartment = useRef(new Compartment());
  const fontCompartment = useRef(new Compartment());

  onChangeRef.current = onChange;
  onSaveRef.current = onSave;

  useEffect(() => {
    if (!host.current) {
      return;
    }
    const state = EditorState.create({
      doc: value,
      extensions: [
        history(),
        drawSelection(),
        highlightActiveLine(),
        bracketMatching(),
        syntaxHighlighting(defaultHighlightStyle, { fallback: true }),
        markdown({ base: markdownLanguage, codeLanguages: languages }),
        EditorView.lineWrapping,
        keymap.of([
          {
            key: "Mod-s",
            run: () => {
              onSaveRef.current?.();
              return true;
            },
          },
          ...defaultKeymap,
          ...historyKeymap,
          indentWithTab,
        ]),
        placeholderExtension(t("placeholder")),
        theme,
        readOnlyCompartment.current.of(EditorState.readOnly.of(readOnly)),
        fontCompartment.current.of(EditorView.theme({ "&": { fontSize: `${fontSize}px` } })),
        EditorView.updateListener.of((update) => {
          if (update.docChanged) {
            onChangeRef.current(update.state.doc.toString());
          }
        }),
      ],
    });
    const instance = new EditorView({ state, parent: host.current });
    view.current = instance;
    return () => {
      instance.destroy();
      view.current = null;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    const instance = view.current;
    if (!instance) {
      return;
    }
    const current = instance.state.doc.toString();
    if (current !== value) {
      instance.dispatch({
        changes: { from: 0, to: current.length, insert: value },
      });
    }
  }, [value]);

  useEffect(() => {
    view.current?.dispatch({
      effects: readOnlyCompartment.current.reconfigure(EditorState.readOnly.of(readOnly)),
    });
  }, [readOnly]);

  useEffect(() => {
    view.current?.dispatch({
      effects: fontCompartment.current.reconfigure(
        EditorView.theme({ "&": { fontSize: `${fontSize}px` } }),
      ),
    });
  }, [fontSize]);

  const wrapSelection = (before: string, after = before, fallback = "") => {
    const instance = view.current;
    if (!instance) {
      return;
    }
    const { from, to } = instance.state.selection.main;
    const selected = instance.state.sliceDoc(from, to) || fallback;
    instance.dispatch({
      changes: { from, to, insert: `${before}${selected}${after}` },
      selection: { anchor: from + before.length, head: from + before.length + selected.length },
    });
    instance.focus();
  };

  const insertBlock = (text: string) => {
    const instance = view.current;
    if (!instance) {
      return;
    }
    const { from } = instance.state.selection.main;
    const prefix = from === 0 ? "" : "\n";
    instance.dispatch({ changes: { from, insert: `${prefix}${text}\n` } });
    instance.focus();
  };

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <div className="flex flex-wrap items-center gap-0.5 border-b border-[var(--border)] px-1.5 py-1">
        <ToolbarButton label={t("toolbar.bold")} onClick={() => wrapSelection("**", "**", "bold")}>
          <Bold size={14} />
        </ToolbarButton>
        <ToolbarButton
          label={t("toolbar.italic")}
          onClick={() => wrapSelection("_", "_", "italic")}
        >
          <Italic size={14} />
        </ToolbarButton>
        <ToolbarButton
          label={t("toolbar.strikethrough")}
          onClick={() => wrapSelection("~~", "~~", "text")}
        >
          <Strikethrough size={14} />
        </ToolbarButton>
        <ToolbarButton label={t("toolbar.code")} onClick={() => wrapSelection("`", "`", "code")}>
          <Code size={14} />
        </ToolbarButton>
        <span className="mx-1 h-4 w-px bg-[var(--border)]" />
        <ToolbarButton label={t("toolbar.heading")} onClick={() => insertBlock("## Heading")}>
          <Heading size={14} />
        </ToolbarButton>
        <ToolbarButton label={t("toolbar.list")} onClick={() => insertBlock("- Item")}>
          <List size={14} />
        </ToolbarButton>
        <ToolbarButton label={t("toolbar.taskList")} onClick={() => insertBlock("- [ ] Task")}>
          <ListChecks size={14} />
        </ToolbarButton>
        <ToolbarButton label={t("toolbar.quote")} onClick={() => insertBlock("> Quote")}>
          <Quote size={14} />
        </ToolbarButton>
        <ToolbarButton label={t("toolbar.codeBlock")} onClick={() => insertBlock("```\n\n```")}>
          <TextQuote size={14} />
        </ToolbarButton>
        <ToolbarButton
          label={t("toolbar.table")}
          onClick={() => insertBlock("| A | B |\n| --- | --- |\n|  |  |")}
        >
          <Table size={14} />
        </ToolbarButton>
        <ToolbarButton
          label={t("toolbar.link")}
          onClick={() => wrapSelection("[", "](https://)", "text")}
        >
          <Link2 size={14} />
        </ToolbarButton>
        <ToolbarButton
          label={t("toolbar.noteLink")}
          onClick={() => wrapSelection("[[", "]]", "Note")}
        >
          <span className="text-[12px] font-semibold">[[</span>
        </ToolbarButton>
        <ToolbarButton label={t("toolbar.math")} onClick={() => wrapSelection("$", "$", "x")}>
          <Sigma size={14} />
        </ToolbarButton>
      </div>
      <div ref={host} className="min-h-0 flex-1 overflow-hidden" spellCheck={spellCheck} />
    </div>
  );
}

function ToolbarButton({
  label,
  onClick,
  children,
}: {
  label: string;
  onClick: () => void;
  children: ReactNode;
}) {
  return (
    <Button
      variant="ghost"
      size="icon"
      className="h-7 w-7"
      title={label}
      aria-label={label}
      onClick={onClick}
    >
      {children}
    </Button>
  );
}
