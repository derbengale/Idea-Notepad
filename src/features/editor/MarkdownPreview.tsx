import { Button } from "@/components/ui/button";
import { Dialog } from "@/components/ui/dialog";
import { useToast } from "@/components/ui/toast";
import type { AttachmentDto } from "@/lib/ipc/types";
import { convertFileSrc } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import { Copy, ImageOff } from "lucide-react";
import { memo, useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { useTranslation } from "react-i18next";
import ReactMarkdown, { defaultUrlTransform, type Components } from "react-markdown";
import rehypeHighlight from "rehype-highlight";
import rehypeKatex from "rehype-katex";
import rehypeSanitize, { defaultSchema } from "rehype-sanitize";
import remarkFrontmatter from "remark-frontmatter";
import remarkGfm from "remark-gfm";
import remarkMath from "remark-math";

export interface MarkdownPreviewProps {
  value: string;
  entryId: string;
  mirrorPath: string;
  allowRemoteImages: boolean;
  sketches: AttachmentDto[];
  onNavigate: (entryId: string) => void;
  onOpenAttachment: (attachmentId: string) => void;
  onOpenSketch: (attachmentId: string) => void;
}

const sanitizeSchema = {
  ...defaultSchema,
  tagNames: [
    "p",
    "br",
    "strong",
    "em",
    "del",
    "code",
    "pre",
    "blockquote",
    "ul",
    "ol",
    "li",
    "a",
    "img",
    "table",
    "thead",
    "tbody",
    "tr",
    "th",
    "td",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "hr",
    "sup",
    "sub",
    "input",
    "span",
    "div",
    "section",
    "math",
    "semantics",
    "annotation",
    "mrow",
    "mi",
    "mo",
    "mn",
    "msup",
    "msub",
    "mfrac",
    "msqrt",
    "mtext",
    "mspace",
    "mtable",
    "mtr",
    "mtd",
    "svg",
    "path",
    "g",
    "use",
    "defs",
    "rect",
    "circle",
    "line",
    "polyline",
    "polygon",
  ],
  attributes: {
    ...(defaultSchema.attributes ?? {}),
    a: ["href", "title", "className", "target", "rel"],
    img: ["src", "alt", "title", "width", "height", "className"],
    input: ["type", "checked", "disabled"],
    code: ["className"],
    span: ["className", "style"],
    div: ["className", "style"],
    section: ["className", "data-footnotes"],
    "*": ["className", "id", "ariaHidden", "aria-label"],
  },
  protocols: {
    ...(defaultSchema.protocols ?? {}),
    href: ["http", "https", "mailto", "note", "attachment", "asset", "file"],
    src: ["http", "https", "asset", "data", "bn-sketch"],
  },
};

function MarkdownPreviewInner({
  value,
  entryId,
  mirrorPath,
  allowRemoteImages,
  sketches,
  onNavigate,
  onOpenAttachment,
  onOpenSketch,
}: MarkdownPreviewProps) {
  const { t } = useTranslation("editor");
  const { push } = useToast();
  const [pendingFile, setPendingFile] = useState<string | null>(null);
  const renderedValue = useMemo(
    () => value.replace(/<!--\s*bn:sketch:([0-9A-Za-z-]+)\s*-->/g, "![sketch](bn-sketch:$1)"),
    [value],
  );

  const resolveImage = (src: string | undefined): string | null => {
    if (!src) {
      return null;
    }
    if (src.startsWith("attachments/")) {
      const normalized = src.replace(/\\/g, "/");
      return convertFileSrc(`${mirrorPath.replace(/\\/g, "/")}/${normalized}`);
    }
    if (src.startsWith("asset:") || src.startsWith("http://asset.localhost")) {
      return src;
    }
    if (/^https?:/i.test(src)) {
      return allowRemoteImages ? src : null;
    }
    if (src.startsWith("data:image/")) {
      return src;
    }
    return null;
  };

  const handleLink = (href: string | undefined) => {
    if (!href) {
      return;
    }
    if (href.startsWith("note:")) {
      onNavigate(href.slice(5));
      return;
    }
    if (href.startsWith("attachment:")) {
      onOpenAttachment(href.slice(11));
      return;
    }
    if (/^https?:/i.test(href) || href.startsWith("mailto:")) {
      void openUrl(href);
      return;
    }
    if (href.startsWith("#")) {
      document.getElementById(href.slice(1))?.scrollIntoView({ behavior: "smooth" });
      return;
    }
    if (href.startsWith("file:")) {
      setPendingFile(href);
    }
  };

  const components = useMemo<Components>(
    () => ({
      a({ href, children, node: _node, ...props }) {
        const isNote = typeof href === "string" && href.startsWith("note:");
        return (
          <a
            {...props}
            href={href}
            title={href}
            className={isNote ? "note-link" : undefined}
            onClick={(event) => {
              event.preventDefault();
              handleLink(href);
            }}
          >
            {children}
          </a>
        );
      },
      img({ src, alt }) {
        if (typeof src === "string" && src.startsWith("bn-sketch:")) {
          const sketch = sketches.find((item) => item.id === src.slice("bn-sketch:".length));
          if (!sketch) {
            return null;
          }
          return <SketchPreviewCard sketch={sketch} onOpen={() => onOpenSketch(sketch.id)} />;
        }
        const resolved = resolveImage(typeof src === "string" ? src : undefined);
        if (!resolved) {
          return (
            <span className="my-2 inline-flex items-center gap-2 rounded-[var(--r-md)] border border-dashed border-[var(--border-strong)] px-3 py-2 text-[12px] text-[var(--text-muted)]">
              <ImageOff size={14} />
              {typeof src === "string" && /^https?:/i.test(src)
                ? t("remoteImageBlocked", { url: src })
                : t("missingAttachment", { name: alt ?? src ?? "" })}
            </span>
          );
        }
        return <img src={resolved} alt={alt ?? ""} loading="lazy" />;
      },
      code({ className, children, node: _node, ...props }) {
        const language = /language-([\w-]+)/.exec(className ?? "")?.[1];
        if (language === "mermaid") {
          return <MermaidBlock chart={String(children).trim()} />;
        }
        return (
          <code className={className} {...props}>
            {children}
          </code>
        );
      },
      pre({ children }) {
        return <CodeBlock>{children}</CodeBlock>;
      },
    }),
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [allowRemoteImages, mirrorPath, entryId, sketches, onOpenSketch],
  );

  return (
    <div className="prose-bn min-h-0 flex-1 overflow-auto px-5 py-4" data-entry-id={entryId}>
      <ReactMarkdown
        remarkPlugins={[remarkFrontmatter, remarkGfm, remarkMath]}
        rehypePlugins={[
          [rehypeSanitize, sanitizeSchema],
          rehypeKatex,
          [rehypeHighlight, { detect: false, ignoreMissing: true }],
        ]}
        urlTransform={(url) => (url.startsWith("bn-sketch:") ? url : defaultUrlTransform(url))}
        components={components}
      >
        {renderedValue}
      </ReactMarkdown>
      <Dialog
        open={pendingFile !== null}
        title={t("confirmLocalFile")}
        onClose={() => setPendingFile(null)}
        footer={
          <>
            <Button
              onClick={() => {
                if (pendingFile) {
                  void navigator.clipboard.writeText(pendingFile.replace(/^file:\/\//, ""));
                  push(t("copied"), { tone: "success" });
                }
                setPendingFile(null);
              }}
            >
              {t("copyPath")}
            </Button>
            <Button
              variant="primary"
              onClick={() => {
                if (pendingFile) {
                  void openUrl(pendingFile);
                }
                setPendingFile(null);
              }}
            >
              {t("openFile")}
            </Button>
          </>
        }
      >
        <p className="break-all font-mono text-[12px]">{pendingFile}</p>
      </Dialog>
    </div>
  );
}

function SketchPreviewCard({ sketch, onOpen }: { sketch: AttachmentDto; onOpen: () => void }) {
  const { t } = useTranslation("editor");
  const source = sketch.previewPath ? convertFileSrc(sketch.previewPath) : null;
  return (
    <span className="my-3 flex w-full flex-col overflow-hidden rounded-[var(--r-md)] border border-[var(--border)] bg-[var(--bg-subtle)]">
      <button
        type="button"
        onClick={onOpen}
        title={t("sketch.open")}
        className="block w-full cursor-zoom-in"
      >
        {source ? (
          <img
            src={source}
            alt={sketch.fileName}
            className="max-h-[420px] w-full object-contain"
            loading="lazy"
          />
        ) : (
          <span className="flex h-32 items-center justify-center text-[12px] text-[var(--text-muted)]">
            {sketch.fileName}
          </span>
        )}
      </button>
      <span className="border-t border-[var(--border)] px-3 py-1.5 text-left text-[11px] text-[var(--text-muted)]">
        {t("sketch.title")}
      </span>
    </span>
  );
}

function CodeBlock({ children }: { children: ReactNode }) {
  const { t } = useTranslation("editor");
  const { push } = useToast();
  const ref = useRef<HTMLPreElement>(null);
  return (
    <div className="group relative">
      <pre ref={ref}>{children}</pre>
      <Button
        variant="ghost"
        size="icon"
        className="absolute right-1 top-1 h-6 w-6 opacity-0 group-hover:opacity-100"
        title={t("copyCode")}
        aria-label={t("copyCode")}
        onClick={() => {
          const text = ref.current?.innerText ?? "";
          void navigator.clipboard.writeText(text);
          push(t("copied"), { tone: "success" });
        }}
      >
        <Copy size={13} />
      </Button>
    </div>
  );
}

function MermaidBlock({ chart }: { chart: string }) {
  const ref = useRef<HTMLDivElement>(null);
  const [failed, setFailed] = useState(false);

  useEffect(() => {
    let cancelled = false;
    void (async () => {
      try {
        const module = await import("mermaid");
        const mermaid = module.default;
        mermaid.initialize({
          startOnLoad: false,
          securityLevel: "strict",
          theme: document.documentElement.dataset.theme === "dark" ? "dark" : "default",
        });
        const id = `bn-mermaid-${Math.random().toString(36).slice(2)}`;
        const result = await mermaid.render(id, chart);
        if (!cancelled && ref.current) {
          ref.current.innerHTML = result.svg;
        }
      } catch {
        if (!cancelled) {
          setFailed(true);
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [chart]);

  if (failed) {
    return <pre>{chart}</pre>;
  }
  return <div ref={ref} className="my-3 flex justify-center overflow-auto" />;
}

export const MarkdownPreview = memo(MarkdownPreviewInner);
export default MarkdownPreview;
