# AGENTS.md — binding rules for every agent in this repository

This file is always in effect, for the orchestrator and for every subagent.

## 1. Read before you write

Before any implementation work, read in this order:

1. `docs/README.md` (index, glossary, conventions)
2. `docs/00-product-vision.md`
3. `docs/01-architecture.md`
4. `docs/02-domain-model.md`
5. `docs/14-orchestration-playbook.md`
6. The specification of your work package (see the table in the playbook)

The documents in `docs/` are the contract. Code that contradicts a document is wrong.
If a document is ambiguous, **stop and report the ambiguity** instead of inventing a requirement.

## 2. Fixed technology decisions

- Shell: **Tauri 2** (Rust core, WebView2 on Windows)
- UI: **React 19 + TypeScript (strict) + Vite + Tailwind CSS + shadcn/ui**
- Markdown: **CodeMirror 6** editor, **unified / react-markdown** preview
- Sketches: **Excalidraw** embedded, stored as `.excalidraw` JSON + PNG preview
- Local storage: **SQLite** via `rusqlite` (bundled, FTS5), database in `%APPDATA%\Better Notepad\`
- Sync: **Google Drive REST v3, `appDataFolder` only**
- i18n: **i18next**, locales `en` (fallback) and `de`
- Target platform: **Windows 10/11**, x64 (Tauri keeps the code portable)

Do not replace, add or upgrade a core technology without an accepted ADR in `docs/adr/`.

## 3. Hard prohibitions

- **No automated tests.** Do not create unit, integration, component, end-to-end or snapshot
  tests, and do not add Vitest, Jest, Playwright, Cypress, pytest or similar dependencies.
  Verification happens through ESLint, Prettier, `tsc --noEmit`, `cargo clippy`, `cargo fmt`
  and the production build. The manual QA checklist belongs to the user.
- **No launching or driving the app.** Never run `npm run tauri dev`, `npm run dev`,
  `npm run dev:web`, the built executable or an installer, and never use browser automation,
  screenshots or clicks to inspect the UI. Agents stop at static gates and builds. The user
  performs all manual and visual confirmation at the very end.
- **No Python.** The legacy PySide6 application was removed; do not reintroduce Python.
- **No full-Drive access.** Never request the `drive` or `drive.readonly` scope. The only Drive
  scope is `https://www.googleapis.com/auth/drive.appdata`.
- **No secrets in the repository.** No client secrets, refresh tokens, API keys or `.env` files
  with real values. Tokens live in the OS credential store.
- **No `eval`-style execution.** Formula evaluation uses the hand-written parser described in
  `docs/04-schema-engine.md`.
- **No silent data loss.** Never overwrite user content without a conflict copy or an explicit
  user decision.
- **No placeholder code.** No `TODO`, `FIXME`, `unimplemented!()` or fake data in shipped paths.

## 4. Language and style

- All specifications, code identifiers, comments and commit messages: **English**.
- All user-visible strings: through i18next keys, present in both `en` and `de`. Never hard-code
  UI text.
- Markdown note content is user data and is never translated.
- TypeScript: `strict`, no `any`, no non-null assertions to silence the compiler.
- Rust: `thiserror` for error enums, no `unwrap`/`expect` in command handlers, `tracing` for logs.

## 5. Data rules

- The Markdown file in Drive is the **source of truth**; SQLite is a local index and offline cache.
- One note = one `.md` file with YAML front matter (`docs/05-markdown-and-files.md`).
- Each note belongs to zero or one project (`docs/08-projects-sharing.md`).
- Timestamps are ISO 8601 UTC with `Z`.
- IDs are UUID v7 strings, never reused, never derived from titles.
- Every write goes through a SQLite transaction and enqueues a sync operation.

## 6. Work-package discipline

- Only touch files inside your assigned ownership paths (playbook, section 4).
- Shared files (`src-tauri/src/commands/mod.rs`, `src/lib/ipc/index.ts`, `src/App.tsx`,
  `src/i18n/locales/*.json`, `src-tauri/tauri.conf.json` for non-packaging work) are owned by the
  orchestrator or by a single named work package. Never edit them as a subagent unless your
  assignment explicitly grants it.
- Finish one work package, report, and stop. Do not continue into the next work package.
- Keep `docs/progress.md` up to date only if the orchestrator asks you to.

## 7. Definition of done for any work package

Static verification is the agent's responsibility. Manual confirmation is exclusively the user's.

1. Files implemented exactly as specified.
2. `npm run lint`, `npm run typecheck` and (when Rust changed) `cargo fmt --check` +
   `cargo clippy -- -D warnings` pass.
3. The app is **not** started and no UI interaction is performed. List the affected QA rows for
   the user; the user confirms the running app at the very end.
4. No new compiler, linter or build warnings. Runtime warnings are checked by the user during the
   final confirmation.
5. UI strings exist in `en` and `de`.
6. Report written in the format defined in `docs/14-orchestration-playbook.md`.

## 8. Commits

Conventional Commits, imperative mood, English, scoped by area:

```
feat(entries): render schema-driven field form
fix(sync): keep conflict copies when revisions diverge
docs(adr): record appDataFolder decision
```

One commit per work package, or per coherent slice inside a work package. Never commit
generated build output or local databases.
