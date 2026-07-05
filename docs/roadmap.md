# novellossless Roadmap

This roadmap follows `novellossless_PRD.md` and tracks verified repository state.

## Product Principles

- Local first: no login, no upload, no AI call by default.
- Original novel text remains authoritative.
- Every extracted candidate, warning, or context item must point back to source text.
- Rust crates own scanning, storage, analysis, and export logic. The desktop UI calls Tauri commands.
- P0 supports only TXT, Markdown, SQLite, offline use, and common Chinese long-form fiction patterns.

## Current Status

Status: P0 local-first foundation implemented with rule-based baseline analysis.

Already verified:

- Tauri 2 desktop shell exists.
- Project import works for explicit local files/directories.
- Scanner stays inside the imported root and does not follow symlinks.
- UTF-8, UTF-8 BOM, and GB18030 text decoding work.
- Chinese chapter splitting and simple English `Chapter N` splitting work.
- SQLite stores projects, documents, and source text fragments.
- Search uses SQLite FTS5 first and escaped `LIKE` fallback for Chinese text.
- Search results show source file, fragment number, and offset range.
- Desktop UI supports import, scan, search, project switching, and a source evidence panel.
- Rule-based person/place/item candidates are generated from local source fragments.
- Explicit foreshadow candidates are generated from local clue/promise markers.
- Basic continuity issues are generated for repeated expressions and simple attribute conflicts.
- Candidate, foreshadow, and issue statuses can be updated.
- Markdown context packs can be generated from source-backed search results.
- Privacy status and local profile information are exposed to the desktop UI.

## P0 Completion Checklist

P0 target from the PRD: prove that an author can import, scan, search, see evidence, review first-pass memory candidates, handle basic warnings, export context, and inspect privacy defaults offline.

| Capability | Status | Notes |
|---|---:|---|
| Cross-platform desktop skeleton | Done | Tauri 2 + React + Vite + Tailwind. |
| Project import | Done | Explicit selected path only. |
| TXT / Markdown scan | Done | Root-bounded, no symlink following. |
| Encoding detection | Done | UTF-8, UTF-8 BOM, GB18030. |
| SQLite storage | Done | Projects, documents, source fragments, FTS. |
| Document / fragment storage | Done | Original text fragments are retained as source evidence. |
| Full-text search | Done | FTS5 + escaped LIKE fallback. |
| Chapter recognition | Done | Chinese web-novel patterns plus simple English chapters. |
| Person / place / item candidates | Done | Rule-based P0 candidates, user-confirmable, not facts. |
| Foreshadow candidates | Done | Explicit clue/promise patterns only. |
| Basic conflict report | Done | Repeated expression and simple attribute checks; advanced semantics remain later. |
| False-positive status management | Done | Status updates for candidates, foreshadows, and issues. |
| Context pack export | Done | Markdown context pack from source-backed search/evidence. |
| Privacy center | Done | Offline defaults and local database visibility. |
| Offline mode | Done | No network or AI required for core loop. |
| Common long-form mode | Done | Local profile manifest is loaded and shown in the UI. |
| Profile loading framework | Done | P0 loads local profile manifest only. |

## Alpha 0: Local Scanning Loop

Goal: a trustworthy local loop for novel text.

Acceptance:

- Import a project directory or single supported file.
- Scan TXT/Markdown without reading outside the root.
- Store documents/fragments in SQLite.
- Search Chinese text and show traceable evidence.
- Display skipped/failed file counts.
- Keep UI language author-friendly.

Exit criteria:

- `cargo fmt`
- `cargo test`
- `pnpm build`
- Rendered desktop UI smoke test.
- Chinese search and evidence display verified.

## Alpha 1: Memory Card Loop

Goal: prove "it remembers" without claiming unverified facts.

Scope:

- Richer person/place/item candidate extraction.
- Candidate cards with source evidence.
- Alias/status fields.
- User confirmation, false-positive, and discarded status.
- Candidate search/filter.

Non-goals:

- AI extraction.
- Strong semantic fact inference.
- Cross-project memory.

## Alpha 2: Conflict And Foreshadow Loop

Goal: surface omissions and contradictions with evidence.

Scope:

- Explicit foreshadow candidates.
- Basic foreshadow ledger.
- Simple attribute contradiction checks.
- Repeated expression checks.
- Issue status management.
- Revision task hooks.

Non-goals:

- Full timeline reasoning.
- Complex character knowledge inference.
- AI explanations.

## Alpha 3: Incremental Scan And Revision History

Goal: enter a real writing workflow.

Scope:

- File watching.
- Debounced incremental scanning.
- Revision event records.
- Hash-based changed-document detection.
- Impact hints from changed source fragments.

## Beta 1: Productized Desktop

Goal: ordinary authors can install and use the app.

Scope:

- Installers.
- System tray.
- Settings.
- Privacy center completion.
- Backup/migration.
- Recovery flows.
- Example project.

## Beta 2: Profile Packs

Goal: mode-specific creative control.

Scope:

- Common long-form mature mode.
- Shuangwen basic mode.
- Historical research basic mode.
- Profile configuration UI.

## v1.0

Goal: stable local-first release.

Scope:

- Full local loop.
- Mature privacy center.
- Report export.
- Context pack export.
- Basic docs/tutorials.
