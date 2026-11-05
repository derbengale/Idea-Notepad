# Better Notepad

A schema-driven, offline-first desktop notepad for ideas, hypotheses, experiments and to-dos.
Notes are plain **Markdown files with YAML front matter**, stored in the user's own private
Google Drive **app data folder** and indexed locally in SQLite.

> **Status: specification phase.**
> There is no application code yet. The app is built by the `Better Notepad Orchestrator`
> agent from the binding specifications in [`docs/`](./docs/README.md).

## What it does

- **Schemas instead of one fixed table.** Every schema defines its own fields and computed
  outputs (text, long text, number, 1–6 scale, coloured status, date, checkbox, tags, formula).
  Schemas are created, renamed, reordered and extended inside the app.
- **Free linking.** Any entry can link to any other entry, regardless of schema; backlinks are
  shown on both sides and can be navigated like a wiki.
- **Markdown-first notes.** Full markdown (formatting, tables, task lists, code, math, images,
  hyperlinks to notes, web URLs and attachments) with a live preview.
- **Infinite canvas sketches.** Drawings are attached to a note and stored as an Excalidraw
  scene plus a PNG preview.
- **Projects.** A note belongs to at most one project; projects group a subset of notes across
  all schemas and can be exported as a self-contained share bundle.
- **Private sync.** OAuth 2.0 with the `drive.appdata` scope only — the app never sees the rest
  of the user's Drive. Files sync device-to-device for the same Google account.
- **German and English UI**, selected from the system language, switchable at runtime.

## Stack

| Layer | Technology |
|---|---|
| Shell | Tauri 2 (Rust) |
| UI | React 19, TypeScript, Vite, Tailwind CSS, shadcn/ui |
| Editor | CodeMirror 6 (source) + unified/react-markdown (preview) |
| Sketch | Excalidraw (MIT) embedded as a view |
| Storage | SQLite (`rusqlite`, bundled, FTS5) as local index/cache |
| Sync | Google Drive REST v3, `appDataFolder` only |
| i18n | i18next (`en`, `de`) |

## Repository layout (target)

```
docs/                     binding specifications and ADRs
.github/agents/           orchestrator + 10 work-package subagents
.github/instructions/     scoped rules for docs, frontend and Rust code
src/                      React frontend (created by WP-01)
src-tauri/                Rust core and Tauri configuration (created by WP-01)
```

## Build (after WP-01)

```
npm install
npm run tauri dev      # development
npm run tauri build    # NSIS + MSI installer
```

## Quality policy

There are **no automated tests** in this project. Quality is enforced by documentation
contracts, TypeScript/Clippy/ESLint gates, a reproducible build and the manual QA checklist in
[`docs/13-quality-and-qa.md`](./docs/13-quality-and-qa.md).

## License

See [LICENSE](./LICENSE).
