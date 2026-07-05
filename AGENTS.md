# AGENTS.md

## Project Principle

novellossless is a local-first novel memory and creative-control assistant for long-form fiction authors.

Correctness means preserving user trust:

- Do not upload novel text by default.
- Do not read outside the imported project root.
- Do not modify original novel files unless explicitly requested.
- Every warning, issue, or extracted claim must be traceable to source text.
- The original novel text layer is authoritative. Summaries, compressed memory, or derived facts must not replace it.

## Required Context

Before non-trivial changes, read the relevant parts of:

- `novellossless_PRD.md`
- `docs/architecture.md`
- The crate or app files you are changing

For product scope, prefer the PRD order:

1. Alpha 0 / P0 local scanning loop
2. Memory card foundation
3. Conflict and foreshadowing loop
4. Incremental scan and revision history
5. Desktop productization
6. AI enhancement

Do not jump ahead to later phases unless the user explicitly asks.

## Current Architecture

The current foundation is a Rust workspace:

- `crates/parser`: text decoding and chapter splitting
- `crates/storage`: SQLite schema, document/chunk persistence, search
- `crates/core`: project import, scan orchestration, search orchestration
- `apps/cli`: local verification entrypoint before the desktop shell exists
- `profiles/common_longform`: first profile manifest placeholder

Keep runtime/storage logic in Rust crates. Do not move storage, scanning, permissions, or replay logic into a future React/Tauri UI unless explicitly required.

## deeplossless Boundary

`D:\deeplossless` is local reference source only.

The dependency target for this project is the published Cargo crate:

```toml
deeplossless = "=0.7.4"
```

Do not use a path dependency to `D:\deeplossless` unless the user explicitly requests local integration testing.

Do not build a parallel memory or storage system when the required behavior is already provided by `deeplossless 0.7.4` and fits the novel domain. If deeplossless behavior does not fit directly, document why and keep the novellossless layer narrow.

Important current distinction: `deeplossless::db::Database::search_unified` is designed for conversation messages, summaries, and snippets. novellossless currently stores novel-specific `document_chunks`, so search follows the deeplossless 0.7.4 strategy (FTS5 first, escaped `LIKE` fallback for CJK) without directly reusing that conversation API.

## Privacy And Filesystem Rules

- Imported project roots must be explicit user-selected files or directories.
- Scanning must stay inside the imported root.
- Do not follow symlinks during scans unless a task explicitly requires and verifies that behavior.
- P0 scanning supports only `.txt`, `.md`, and `.markdown`.
- Never silently skip unreadable files in user-facing flows; report skipped/failed files.
- Generated databases and build artifacts must not be treated as user novel source.

## Editing Rules

- Make the smallest change that satisfies the request.
- Match the existing module boundaries.
- Do not refactor unrelated code.
- Check all callers before changing public functions, shared types, schema fields, or CLI behavior.
- Remove imports, variables, and functions made unused by your change.
- Prefer explicit errors over silent degradation.

## Search And Text Handling

Chinese text is a first-class requirement.

- Keep UTF-8, UTF-8 BOM, and GB18030/GBK-compatible decoding working.
- Keep chapter recognition focused on common Chinese web-novel patterns plus simple English `Chapter N`.
- SQLite FTS5 alone is not enough for Chinese matching. Use escaped `LIKE` fallback or a verified tokenizer upgrade.
- Treat `%`, `_`, and `\` in user search queries as literal text unless the user explicitly asks for pattern syntax.

## UI Work

When UI work starts, product quality is part of correctness.

- Build a coherent desktop product surface, not a collection of patched controls.
- Keep UI language for ordinary authors; avoid raw technical terms such as DAG, embedding, vector, chunk, semantic hash, LCM, and token pressure.
- Do not expose raw local paths where a basename or project-relative path is enough.
- Verify rendered UI with screenshots or browser inspection when practical.

## Verification

For Rust changes, run:

```powershell
cargo fmt
cargo test
```

When touching the deeplossless dependency boundary, also run:

```powershell
cargo test -p novellossless-core --features deeplossless-compat
```

When relevant, verify failure paths, skipped files, state consistency after rescans, and Chinese search behavior. If something was not verified, say so explicitly.

## Reporting

In final reports, include:

- What changed
- What was verified
- What remains unverified or intentionally out of scope

Do not claim correctness beyond the behavior actually tested.
