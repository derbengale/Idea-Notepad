import "@excalidraw/excalidraw/index.css";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import "highlight.js/styles/github.css";
import "katex/dist/katex.min.css";
import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { ErrorBoundary } from "./components/app/ErrorBoundary";
import { ToastProvider } from "./components/ui/toast";
import "./i18n";
import "./styles/globals.css";

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      refetchOnWindowFocus: false,
      retry: false,
      staleTime: 5_000,
    },
  },
});

const root = document.getElementById("root");

if (!root) {
  throw new Error("Root element is missing");
}

ReactDOM.createRoot(root).render(
  <React.StrictMode>
    <ErrorBoundary>
      <QueryClientProvider client={queryClient}>
        <ToastProvider>
          <App />
        </ToastProvider>
      </QueryClientProvider>
    </ErrorBoundary>
  </React.StrictMode>,
);
