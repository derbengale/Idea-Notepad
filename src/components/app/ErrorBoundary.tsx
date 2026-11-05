import { Button } from "@/components/ui/button";
import { Component, type ErrorInfo, type ReactNode } from "react";
import { useTranslation } from "react-i18next";

function ErrorFallback({ code }: { code: string }) {
  const { t } = useTranslation("errors");
  return (
    <div className="flex h-full flex-col items-center justify-center gap-3 bg-[var(--bg)] p-8 text-center">
      <h1 className="text-[18px] font-semibold">{t("UNKNOWN.title")}</h1>
      <p className="max-w-[420px] text-[13px] text-[var(--text-muted)]">
        {t("UNKNOWN.description")}
      </p>
      <code className="max-w-[520px] overflow-auto rounded bg-[var(--bg-subtle)] px-2 py-1 text-[11px]">
        {code}
      </code>
      <Button variant="primary" onClick={() => window.location.reload()}>
        {t("details")}
      </Button>
    </div>
  );
}

export class ErrorBoundary extends Component<{ children: ReactNode }, { message: string | null }> {
  state: { message: string | null } = { message: null };

  static getDerivedStateFromError(error: unknown): { message: string } {
    return { message: error instanceof Error ? error.message : String(error) };
  }

  componentDidCatch(error: Error, info: ErrorInfo): void {
    console.error("Unhandled UI error", error, info.componentStack);
  }

  render(): ReactNode {
    if (this.state.message) {
      return <ErrorFallback code={this.state.message} />;
    }
    return this.props.children;
  }
}
