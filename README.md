# novellossless

Local-first novel memory and continuity assistant for long-form Chinese fiction.

The initial foundation implements the Alpha 0 loop from `novellossless_PRD.md`: import a local project, scan TXT/Markdown files, split chapters, persist documents/chunks in SQLite, and search with SQLite FTS5.

## Verify locally

```powershell
cargo test
cargo run -p novellossless-cli -- --db .\novellossless.db init
```

## Desktop app

The desktop shell lives in `apps/desktop` and uses Tauri 2, React, Vite, and Tailwind CSS.

```powershell
cd apps\desktop
pnpm install
pnpm dev
```

The Vite dev server uses `http://127.0.0.1:5180/`. Browser preview shows seed data when Tauri commands are unavailable; the packaged/dev Tauri shell calls the Rust core commands for project import, scanning, search, and summaries.

## Dependency boundary

`deeplossless` is pinned to the published crate version `=0.7.4`. The local `D:\deeplossless` source tree is only a reference checkout unless a future task explicitly asks for local path dependency testing.
