# novellossless Architecture Notes

This project starts with the Alpha 0 local scanning loop from `novellossless_PRD.md`.

## Current foundation

- `crates/parser`: reads UTF-8, UTF-8 BOM, and GB18030 text; splits TXT/Markdown content into chapter chunks.
- `crates/storage`: owns SQLite schema, project records, document/chunk storage, and FTS5 search.
- `crates/core`: coordinates project import, TXT/Markdown scanning, hashing, chunk persistence, search, P0 rule-based analysis, and context-pack generation.
- `apps/cli`: a verification entrypoint for local init/import/scan/search before the desktop shell exists.
- `apps/desktop`: Tauri 2 desktop shell with a React/Vite/Tailwind product workspace.
- `profiles/common_longform`: the first profile manifest placeholder for the P0 mode package framework.

## P0 analysis layer

The P0 analysis layer is intentionally conservative:

- Person/place/item records are candidates, not facts.
- Foreshadow records are only explicit clue or promise candidates.
- Continuity issues are low-to-medium confidence warnings with source evidence.
- User status updates are stored locally so false positives can be marked without modifying novel files.
- Context packs are Markdown outputs assembled from source-backed search results.

This layer does not call AI and does not replace the original text layer.

## deeplossless boundary

`deeplossless` is pinned as a published Cargo dependency with `=0.7.4`.
The local `D:\deeplossless` checkout is reference material only and is not used as a path dependency.

novellossless stores an independent original text and document chunk layer. Compressed or derived memory must not become the only source of novel text.

SQLite search follows the `deeplossless 0.7.4` pattern: try FTS5 first for tokenized queries, then use escaped `LIKE` as the reliable fallback for CJK and mixed Chinese/English text. The current tables are novel-specific (`document_chunks`), so `deeplossless::db::Database::search_unified` is not called directly.

## Privacy assumptions

- Project import accepts only an existing file or directory selected by the user.
- Scanning walks inside that root and does not follow symlinks.
- P0 scanning only indexes `.txt`, `.md`, and `.markdown`.
- The CLI does not call AI, read the clipboard, capture the screen, or access files outside the imported root.
- The desktop shell uses Tauri commands as its boundary to Rust core logic. React must not bypass that boundary to read SQLite or project files directly.
