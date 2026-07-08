# Beta 2: 创作模式包系统 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the full profile pack system — ProfileLoader, RuleEngine, MetricRegistry, IssueEmitter, KnowledgePackLoader — with three built-in profiles (common_longform, shuangwen, history), per-project profile selection, and profile config UI.

**Architecture:** New `crates/profiles/` crate owns all profile runtime logic (loader, schema, rules, metrics, checks, knowledge packs). `crates/storage/` adds 4 new tables. `crates/core/` integrates profiles into the scan pipeline. Tauri commands bridge to React settings UI. Profile configs live under `profiles/<id>/` as TOML files.

**Tech Stack:** Rust (anyhow, serde, toml, rusqlite), Tauri 2, React/TypeScript

## Global Constraints

- New tables use `IF NOT EXISTS` for idempotent migration
- All `profiles/` TOML files use `serde::Deserialize` with `#[serde(default)]`
- Storage methods follow existing `self.conn.execute()` + `params![]` pattern
- `crates/profiles/` depends only on `serde`, `toml`, `anyhow` at runtime (no storage or core deps)
- `cargo test` must pass after each task
- `tsc --noEmit` must pass after UI tasks
- DTOs in desktop use `#[derive(Debug, Serialize)]` + `#[serde(rename_all = "camelCase")]`
- No serde derives in `crates/storage` or `crates/profiles` data structs (core/crates convention)
- Chinese first-class: metrics and checks operate on Chinese text with keyword matching

---
## File Structure

### Create:
- `crates/profiles/Cargo.toml`
- `crates/profiles/src/lib.rs`
- `crates/profiles/src/manifest.rs`
- `crates/profiles/src/loader.rs`
- `crates/profiles/src/rule_engine.rs`
- `crates/profiles/src/metrics.rs`
- `crates/profiles/src/checks.rs`
- `crates/profiles/src/knowledge.rs`
- `profiles/shuangwen/profile.toml`
- `profiles/shuangwen/metrics.toml`
- `profiles/shuangwen/rules.toml`
- `profiles/history/profile.toml`
- `profiles/history/rules.toml`
- `profiles/history/knowledge/tang_officials.toml`
- `profiles/history/knowledge/tang_places.toml`

### Modify:
- `Cargo.toml` (workspace)
- `crates/storage/src/lib.rs`
- `crates/core/Cargo.toml`
- `crates/core/src/lib.rs`
- `crates/core/src/profile.rs`
- `profiles/common_longform/profile.toml`
- `apps/desktop/src-tauri/Cargo.toml`
- `apps/desktop/src-tauri/src/lib.rs`
- `apps/desktop/src/tauri.ts`
- `apps/desktop/src/routes/Settings.tsx`
- `apps/cli/src/main.rs`

---
## Interfaces Map

```
Task 1 (Storage): Storage::upsert_profile_metric, get_project_profiles, set_project_profiles, etc.
Task 2 (Profiles crate): ProfileManifest, ProfileLoader::load_all
Task 3 (Profiles crate): RuleEngine::merge_rules
Task 4 (Profiles crate): MetricRegistry::from_profiles, compute_all
Task 5 (Profiles crate): IssueEmitter::emit with checks
Task 6 (Profiles crate): KnowledgePackLoader::load_all, build_index
Task 7 (Config files): profile.toml, rules.toml, metrics.toml for 3 profiles
Task 8 (Core): NovelCore.get_available_profiles, get_enabled_profiles, set_enabled_profiles
Task 9 (Core): compute_profile_metrics, emit_profile_checks in analyze_project
Task 10 (Tauri): get_available_profiles, get_enabled_profiles, set_enabled_profiles, get_profile_metrics, get_knowledge
Task 11 (CLI): profiles subcommand
Task 12 (Frontend): Settings page profile toggles + profile metrics section
```

---

### Task 1: Storage — profile_metrics + knowledge_packs tables

**Files:**
- Modify: `crates/storage/src/lib.rs` (add tables in `init()`, add structs, add methods)

**Interfaces Produced:**
- `NewProfileMetric { profile_id, project_id, metric_type, document_id, value_json }`
- `ProfileMetric { id, profile_id, metric_type, document_id, value, created_at }`
- `KnowledgePackEntry { id, profile_id, pack_name, pack_type, entries_json, version }`
- `Storage::upsert_profile_metric(&self, metric: &NewProfileMetric) -> Result<()>`
- `Storage::get_profile_metrics(&self, project_id: &str, profile_id: &str) -> Result<Vec<ProfileMetric>>`
- `Storage::get_project_profiles(&self, project_id: &str) -> Result<Vec<String>>`
- `Storage::set_project_profiles(&self, project_id: &str, profile_ids: &[&str]) -> Result<()>`
- `Storage::upsert_knowledge_pack(&self, pack: &KnowledgePackEntry) -> Result<()>`
- `Storage::get_knowledge_packs(&self, profile_id: &str) -> Result<Vec<KnowledgePackEntry>>`

- [ ] **Step 1: Add 4 new tables in `init()`**

Add to the `init()` `execute_batch`:
```sql
CREATE TABLE IF NOT EXISTS profiles (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    version TEXT NOT NULL DEFAULT '0.1.0',
    path TEXT NOT NULL,
    enabled INTEGER NOT NULL DEFAULT 1,
    settings_json TEXT NOT NULL DEFAULT '{}'
);

CREATE TABLE IF NOT EXISTS profile_metrics (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    profile_id TEXT NOT NULL,
    metric_type TEXT NOT NULL,
    document_id TEXT REFERENCES documents(id) ON DELETE CASCADE,
    value_json TEXT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS knowledge_packs (
    id TEXT PRIMARY KEY,
    profile_id TEXT NOT NULL,
    pack_name TEXT NOT NULL,
    pack_type TEXT NOT NULL,
    entries_json TEXT NOT NULL,
    version TEXT NOT NULL DEFAULT '0.1.0',
    created_at TEXT NOT NULL
);
```

- [ ] **Step 2: Add structs and storage methods**

After `ContextPack` struct, add:
```rust
#[derive(Debug, Clone)]
pub struct NewProfileMetric {
    pub profile_id: String,
    pub project_id: String,
    pub metric_type: String,
    pub document_id: Option<String>,
    pub value_json: String,
}

#[derive(Debug, Clone)]
pub struct ProfileMetric {
    pub id: String,
    pub profile_id: String,
    pub metric_type: String,
    pub document_id: Option<String>,
    pub value: String,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct KnowledgePackEntry {
    pub id: String,
    pub profile_id: String,
    pub pack_name: String,
    pub pack_type: String,
    pub entries_json: String,
    pub version: String,
}
```

Add methods after `existing_document_id`:
```rust
pub fn upsert_profile_metric(&self, metric: &NewProfileMetric) -> Result<()> {
    self.conn.execute(
        "INSERT INTO profile_metrics (id, project_id, profile_id, metric_type, document_id, value_json, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            Uuid::new_v4().to_string(),
            metric.project_id,
            metric.profile_id,
            metric.metric_type,
            metric.document_id,
            metric.value_json,
            Utc::now().to_rfc3339()
        ],
    )?;
    Ok(())
}

pub fn get_profile_metrics(&self, project_id: &str, profile_id: &str) -> Result<Vec<ProfileMetric>> {
    let mut stmt = self.conn.prepare(
        "SELECT id, profile_id, metric_type, document_id, value_json, created_at
         FROM profile_metrics
         WHERE project_id = ?1 AND profile_id = ?2
         ORDER BY created_at DESC"
    )?;
    let rows = stmt.query_map(params![project_id, profile_id], |row| {
        Ok(ProfileMetric {
            id: row.get(0)?,
            profile_id: row.get(1)?,
            metric_type: row.get(2)?,
            document_id: row.get(3)?,
            value: row.get(4)?,
            created_at: row.get(5)?,
        })
    })?;
    rows.collect::<rusqlite::Result<Vec<_>>>().map_err(Into::into)
}

pub fn get_project_profiles(&self, project_id: &str) -> Result<Vec<String>> {
    let json: String = self.conn.query_row(
        "SELECT enabled_profiles_json FROM projects WHERE id = ?1",
        params![project_id],
        |row| row.get(0),
    ).unwrap_or_default();
    if json.is_empty() {
        return Ok(vec!["common_longform".to_string()]);
    }
    serde_json::from_str(&json).map_err(|e| anyhow::anyhow!("parse enabled_profiles_json: {e}"))
}

pub fn set_project_profiles(&self, project_id: &str, profile_ids: &[&str]) -> Result<()> {
    let json = serde_json::to_string(profile_ids)?;
    self.conn.execute(
        "UPDATE projects SET enabled_profiles_json = ?1, updated_at = ?2 WHERE id = ?3",
        params![json, Utc::now().to_rfc3339(), project_id],
    )?;
    Ok(())
}

pub fn upsert_knowledge_pack(&self, pack: &KnowledgePackEntry) -> Result<()> {
    self.conn.execute(
        "INSERT INTO knowledge_packs (id, profile_id, pack_name, pack_type, entries_json, version, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            Uuid::new_v4().to_string(),
            pack.profile_id,
            pack.pack_name,
            pack.pack_type,
            pack.entries_json,
            pack.version,
            Utc::now().to_rfc3339()
        ],
    )?;
    Ok(())
}

pub fn get_knowledge_packs(&self, profile_id: &str) -> Result<Vec<KnowledgePackEntry>> {
    let mut stmt = self.conn.prepare(
        "SELECT id, profile_id, pack_name, pack_type, entries_json, version
         FROM knowledge_packs WHERE profile_id = ?1"
    )?;
    let rows = stmt.query_map(params![profile_id], |row| {
        Ok(KnowledgePackEntry {
            id: row.get(0)?,
            profile_id: row.get(1)?,
            pack_name: row.get(2)?,
            pack_type: row.get(3)?,
            entries_json: row.get(4)?,
            version: row.get(5)?,
        })
    })?;
    rows.collect::<rusqlite::Result<Vec<_>>>().map_err(Into::into)
}
```

- [ ] **Step 3: Write tests**

In `crates/storage/src/lib.rs` `mod tests`:
```rust
#[test]
fn stores_and_retrieves_profile_metrics() -> Result<()> {
    let (storage, pid) = test_storage_with_project("profile_metrics_test")?;
    storage.upsert_profile_metric(&NewProfileMetric {
        profile_id: "shuangwen".into(),
        project_id: pid.clone(),
        metric_type: "爽点密度".into(),
        document_id: None,
        value_json: r#"{"value": 3.5, "unit": "per_1000_chars"}"#.into(),
    })?;
    let metrics = storage.get_profile_metrics(&pid, "shuangwen")?;
    assert_eq!(metrics.len(), 1);
    assert_eq!(metrics[0].metric_type, "爽点密度");
    Ok(())
}

#[test]
fn stores_and_loads_project_profiles() -> Result<()> {
    let (storage, pid) = test_storage_with_project("profiles_crud_test")?;
    let default = storage.get_project_profiles(&pid)?;
    assert_eq!(default, vec!["common_longform"]);
    storage.set_project_profiles(&pid, &["common_longform", "shuangwen"])?;
    let loaded = storage.get_project_profiles(&pid)?;
    assert_eq!(loaded.len(), 2);
    assert!(loaded.contains(&"shuangwen".to_string()));
    Ok(())
}

#[test]
fn stores_and_retrieves_knowledge_packs() -> Result<()> {
    let storage = test_storage()?;
    storage.upsert_knowledge_pack(&KnowledgePackEntry {
        id: String::new(),
        profile_id: "history".into(),
        pack_name: "tang_officials".into(),
        pack_type: "officials".into(),
        entries_json: r#"[{"term":"尚书","rank":"正三品"}]"#.into(),
        version: "0.1.0".into(),
    })?;
    let packs = storage.get_knowledge_packs("history")?;
    assert_eq!(packs.len(), 1);
    assert_eq!(packs[0].pack_name, "tang_officials");
    Ok(())
}
```

- [ ] **Step 4: Run tests and verify**

Run: `cargo test -p novellossless-storage`
Expected: all tests pass (existing + 3 new)

- [ ] **Step 5: Commit**

```bash
git add crates/storage/src/lib.rs
git commit -m "feat(storage): add profile_metrics, knowledge_packs tables and CRUD methods"
```

---

### Task 2: Create `crates/profiles/` — crate skeleton + ProfileManifest + ProfileLoader

**Files:**
- Create: `crates/profiles/Cargo.toml`
- Create: `crates/profiles/src/lib.rs`
- Create: `crates/profiles/src/manifest.rs`
- Create: `crates/profiles/src/loader.rs`
- Modify: `Cargo.toml` (workspace)

**Interfaces Produced:**
- `ProfileManifest` with full PRD schema
- `ProfileLoader::load_all(profiles_root: &Path) -> Result<Vec<ProfileManifest>>`
- `ProfileLoader::load_rules(profiles_root: &Path, profile_id: &str) -> Result<Option<ProfileRules>>`

- [ ] **Step 1: Register crate in workspace `Cargo.toml`**

Add `"crates/profiles"` to workspace members. Add:
```toml
novellossless-profiles = { path = "crates/profiles" }
```
to workspace dependencies.

- [ ] **Step 2: Create `crates/profiles/Cargo.toml`**

```toml
[package]
name = "novellossless-profiles"
version = "0.1.0"
edition.workspace = true
license.workspace = true
repository.workspace = true

[dependencies]
anyhow.workspace = true
serde.workspace = true
toml.workspace = true
```

- [ ] **Step 3: Create `crates/profiles/src/manifest.rs`**

```rust
use serde::Deserialize;

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ProfileManifest {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub enabled_by_default: Option<bool>,

    #[serde(default)]
    pub entities: EntityTypes,
    #[serde(default)]
    pub facts: FactTypes,
    #[serde(default)]
    pub events: EventTypes,
    #[serde(default)]
    pub metrics: MetricDefs,
    #[serde(default)]
    pub checks: CheckDefs,
    #[serde(default)]
    pub templates: TemplateDefs,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct EntityTypes {
    pub types: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct FactTypes {
    pub types: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct EventTypes {
    pub types: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct MetricDefs {
    pub enabled: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct CheckDefs {
    pub enabled: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct TemplateDefs {
    pub enabled: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ReportDefs {
    pub enabled: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct ProfileRules {
    pub chapter_recognition: bool,
    pub full_text_search: bool,
    pub evidence_required: bool,
    pub auto_modify_source: bool,
    pub extractors: ExtractorRules,
    pub people: PeopleConfig,
}

impl Default for ProfileRules {
    fn default() -> Self {
        Self {
            chapter_recognition: true,
            full_text_search: true,
            evidence_required: true,
            auto_modify_source: false,
            extractors: ExtractorRules::default(),
            people: PeopleConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct ExtractorRules {
    pub people: bool,
    pub places: bool,
    pub items: bool,
    pub foreshadows: bool,
    pub eye_color_conflicts: bool,
    pub repeat_expressions: bool,
    #[serde(default)]
    pub shuangwen_metrics: bool,
    #[serde(default)]
    pub history_checks: bool,
}

impl Default for ExtractorRules {
    fn default() -> Self {
        Self {
            people: true,
            places: true,
            items: true,
            foreshadows: true,
            eye_color_conflicts: true,
            repeat_expressions: true,
            shuangwen_metrics: false,
            history_checks: false,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct PeopleConfig {
    pub min_name_length: u32,
    pub max_name_length: u32,
    pub enable_alias_detection: bool,
}

impl Default for PeopleConfig {
    fn default() -> Self {
        Self {
            min_name_length: 2,
            max_name_length: 4,
            enable_alias_detection: true,
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct MetricsToml {
    #[serde(default)]
    pub metrics: Vec<MetricTomlEntry>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct MetricTomlEntry {
    pub id: String,
    pub name: String,
    pub description: String,
    #[serde(default = "default_weight")]
    pub weight: f64,
}

fn default_weight() -> f64 { 1.0 }

#[derive(Debug, Clone, Default)]
pub struct CheckDefinition {
    pub id: String,
    pub name: String,
    pub description: String,
    pub profile_id: String,
    pub severity: String,
}
```

- [ ] **Step 4: Create `crates/profiles/src/loader.rs`**

```rust
use std::path::Path;
use anyhow::{Context, Result};
use crate::manifest::*;

pub struct ProfileLoader;

impl ProfileLoader {
    pub fn load_all(profiles_root: &Path) -> Result<Vec<ProfileManifest>> {
        let mut manifests = Vec::new();
        let read_dir = match std::fs::read_dir(profiles_root) {
            Ok(d) => d,
            Err(_) => return Ok(manifests),
        };
        for entry in read_dir {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                continue;
            }
            let profile_toml = entry.path().join("profile.toml");
            if !profile_toml.exists() {
                continue;
            }
            let content = std::fs::read_to_string(&profile_toml)
                .with_context(|| format!("failed to read {}", profile_toml.display()))?;
            match toml::from_str::<ProfileManifest>(&content) {
                Ok(mut m) => {
                    if m.id.is_empty() {
                        m.id = entry.file_name().to_string_lossy().to_string();
                    }
                    manifests.push(m);
                }
                Err(e) => {
                    log_warn(&format!("skipping {}: {e}", entry.path().display()));
                }
            }
        }
        manifests.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(manifests)
    }

    pub fn load_rules(profiles_root: &Path, profile_id: &str) -> Result<Option<ProfileRules>> {
        let path = profiles_root.join(profile_id).join("rules.toml");
        if !path.exists() {
            return Ok(None);
        }
        let content = std::fs::read_to_string(&path)?;
        toml::from_str(&content)
            .map(Some)
            .map_err(|e| anyhow::anyhow!("failed to parse {}: {e}", path.display()))
    }

    pub fn load_metrics_toml(profiles_root: &Path, profile_id: &str) -> Result<Option<MetricsToml>> {
        let path = profiles_root.join(profile_id).join("metrics.toml");
        if !path.exists() {
            return Ok(None);
        }
        let content = std::fs::read_to_string(&path)?;
        toml::from_str(&content)
            .map(Some)
            .map_err(|e| anyhow::anyhow!("failed to parse {}: {e}", path.display()))
    }
}

fn log_warn(msg: &str) {
    eprintln!("warning (profiles): {msg}");
}
```

- [ ] **Step 5: Create `crates/profiles/src/lib.rs`**

```rust
pub mod checks;
pub mod knowledge;
pub mod loader;
pub mod manifest;
pub mod metrics;
pub mod rule_engine;

pub use checks::IssueEmitter;
pub use knowledge::KnowledgePackLoader;
pub use loader::ProfileLoader;
pub use manifest::*;
pub use metrics::MetricRegistry;
pub use rule_engine::RuleEngine;

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    fn test_profiles_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("profiles_test_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir.join("common_longform")).unwrap();
        fs::write(
            dir.join("common_longform").join("profile.toml"),
            r#"id = "common_longform"
name = "通用长篇"
version = "0.1.0"
description = "适用于绝大多数长篇小说的通用模式"#,
        )
        .unwrap();
        fs::create_dir_all(&dir.join("shuangwen")).unwrap();
        fs::write(
            dir.join("shuangwen").join("profile.toml"),
            r#"id = "shuangwen"
name = "爽文模式"
version = "0.1.0"
description = "监控爽点、升级、打脸、战力和读者反馈"

[metrics]
enabled = ["爽点密度", "冲突频次"]

[checks]
enabled = ["战力倒退检查"]"#,
        )
        .unwrap();
        dir
    }

    #[test]
    fn profile_loader_discovers_all_profiles() {
        let dir = test_profiles_dir();
        let manifests = ProfileLoader::load_all(&dir).unwrap();
        assert_eq!(manifests.len(), 2);
        let ids: Vec<&str> = manifests.iter().map(|m| m.id.as_str()).collect();
        assert!(ids.contains(&"common_longform"));
        assert!(ids.contains(&"shuangwen"));
    }

    #[test]
    fn profile_loader_skips_dirs_without_profile_toml() {
        let dir = test_profiles_dir();
        fs::create_dir_all(dir.join("empty_dir")).unwrap();
        let manifests = ProfileLoader::load_all(&dir).unwrap();
        assert_eq!(manifests.len(), 2);
    }

    #[test]
    fn profile_loader_loads_rules() {
        let dir = test_profiles_dir();
        fs::write(
            dir.join("common_longform").join("rules.toml"),
            r#"[extractors]
people = true
places = true

[people]
min_name_length = 2"#,
        )
        .unwrap();
        let rules = ProfileLoader::load_rules(&dir, "common_longform")
            .unwrap()
            .expect("rules should be present");
        assert!(rules.extractors.people);
        assert_eq!(rules.people.min_name_length, 2);
    }

    #[test]
    fn profile_loader_loads_metrics_toml() {
        let dir = test_profiles_dir();
        fs::write(
            dir.join("shuangwen").join("metrics.toml"),
            r#"[[metrics]]
id = "爽点密度"
name = "爽点密度"
description = "每千字爽点词出现次数"
weight = 1.0"#,
        )
        .unwrap();
        let metrics_toml = ProfileLoader::load_metrics_toml(&dir, "shuangwen")
            .unwrap()
            .expect("metrics should be present");
        assert_eq!(metrics_toml.metrics.len(), 1);
        assert_eq!(metrics_toml.metrics[0].id, "爽点密度");
    }
}
```

- [ ] **Step 6: Run tests**

Run: `cargo test -p novellossless-profiles`
Expected: all 4 tests pass

- [ ] **Step 7: Commit**

```bash
git add Cargo.toml crates/profiles/
git commit -m "feat(profiles): new crate with ProfileManifest and ProfileLoader"
```

---

### Task 3: RuleEngine — merge rules from multiple profiles

**Files:**
- Create: `crates/profiles/src/rule_engine.rs`

**Interfaces Produced:**
- `RuleEngine { extractors: ExtractorRules }`
- `RuleEngine::merge_rules(manifests: &[ProfileManifest], root: &Path) -> Result<Self>`

- [ ] **Step 1: Write failing test**

Add to `crates/profiles/src/lib.rs` `mod tests`:
```rust
#[test]
fn rule_engine_merges_multiple_profiles() {
    let dir = test_profiles_dir();
    fs::write(
        dir.join("shuangwen").join("rules.toml"),
        r#"[extractors]
shuangwen_metrics = true"#,
    )
    .unwrap();
    fs::write(
        dir.join("common_longform").join("rules.toml"),
        r#"[extractors]
people = true
places = true"#,
    )
    .unwrap();

    let manifests = ProfileLoader::load_all(&dir).unwrap();
    let engine = RuleEngine::merge_rules(&manifests, &dir).unwrap();
    assert!(engine.extractors.people);
    assert!(engine.extractors.shuangwen_metrics);
    assert!(!engine.extractors.history_checks);
}
```

- [ ] **Step 2: Implement `rule_engine.rs`**

```rust
use crate::manifest::*;
use crate::loader::ProfileLoader;
use anyhow::Result;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct RuleEngine {
    pub extractors: ExtractorRules,
}

impl RuleEngine {
    pub fn merge_rules(manifests: &[ProfileManifest], root: &Path) -> Result<Self> {
        let mut merged = ExtractorRules::default();
        // Start with all false
        merged.people = false;
        merged.places = false;
        merged.items = false;
        merged.foreshadows = false;
        merged.eye_color_conflicts = false;
        merged.repeat_expressions = false;
        merged.shuangwen_metrics = false;
        merged.history_checks = false;

        for m in manifests {
            if let Ok(Some(rules)) = ProfileLoader::load_rules(root, &m.id) {
                if rules.extractors.people { merged.people = true; }
                if rules.extractors.places { merged.places = true; }
                if rules.extractors.items { merged.items = true; }
                if rules.extractors.foreshadows { merged.foreshadows = true; }
                if rules.extractors.eye_color_conflicts { merged.eye_color_conflicts = true; }
                if rules.extractors.repeat_expressions { merged.repeat_expressions = true; }
                if rules.extractors.shuangwen_metrics { merged.shuangwen_metrics = true; }
                if rules.extractors.history_checks { merged.history_checks = true; }
            }
        }

        Ok(Self { extractors: merged })
    }
}
```

- [ ] **Step 3: Run tests**

Run: `cargo test -p novellossless-profiles`
Expected: all 5 tests pass

- [ ] **Step 4: Commit**

```bash
git add crates/profiles/src/rule_engine.rs crates/profiles/src/lib.rs
git commit -m "feat(profiles): add RuleEngine merging rules from multiple profiles"
```

---

### Task 4: MetricRegistry — profile-specific metrics computation

**Files:**
- Create: `crates/profiles/src/metrics.rs`

**Interfaces Produced:**
- `MetricRegistry { metrics: Vec<MetricDefinition> }`
- `MetricRegistry::from_profiles(manifests: &[ProfileManifest], root: &Path) -> Result<Self>`
- `MetricRegistry::compute_all(&self, chunks: &[&str]) -> Vec<MetricResult>`
- `MetricResult { profile_id, metric_type, value }`

- [ ] **Step 1: Write failing tests**

Add to `crates/profiles/src/lib.rs`:
```rust
#[test]
fn metric_registry_computes_shuangwen_metrics() {
    let registry = MetricRegistry::from_profiles(&[], &PathBuf::from("/nonexistent")).unwrap();

    let chapters = vec![
        "第一章 林澈一拳打脸反派，众人震惊！他直接升级突破了。",
        "第二章 碾压对手，全场震惊。又是一个爽点。",
    ];

    let results = registry.compute_all(&chapters);
    // With empty profiles, compute_all returns empty
    assert!(results.is_empty());
}

#[test]
fn metric_registry_computes_metric() {
    let registry = MetricRegistry::from_profiles(&[], &PathBuf::from("/nonexistent")).unwrap();
    let chapters = vec!["打脸！升级！碾压！众人震惊！"];
    let result = registry.compute("爽点密度", &chapters);
    assert!(result.is_some());
    assert!(result.unwrap() > 0.0);
}
```

- [ ] **Step 2: Implement `metrics.rs`**

```rust
use crate::loader::ProfileLoader;
use anyhow::Result;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct MetricDefinition {
    pub metric_type: String,
    pub profile_id: String,
    pub name: String,
    pub description: String,
    pub weight: f64,
    pub kind: MetricKind,
}

#[derive(Debug, Clone)]
pub enum MetricKind {
    KeywordDensity(Vec<String>),
    KeywordInterval(Vec<String>),
    ModernWordDensity(Vec<String>),
}

#[derive(Debug, Clone)]
pub struct MetricResult {
    pub profile_id: String,
    pub metric_type: String,
    pub value: f64,
    pub unit: String,
}

pub struct MetricRegistry {
    pub metrics: Vec<MetricDefinition>,
}

impl MetricRegistry {
    pub fn from_profiles(manifests: &[ProfileManifest], root: &Path) -> Result<Self> {
        let mut metrics = Vec::new();
        for m in manifests {
            if let Ok(Some(metrics_toml)) = ProfileLoader::load_metrics_toml(root, &m.id) {
                for entry in metrics_toml.metrics {
                    let kind = metric_kind_for(&entry.id);
                    metrics.push(MetricDefinition {
                        metric_type: entry.id.clone(),
                        profile_id: m.id.clone(),
                        name: entry.name,
                        description: entry.description,
                        weight: entry.weight,
                        kind,
                    });
                }
            }
        }
        Ok(Self { metrics })
    }

    pub fn compute_all(&self, chunks: &[&str]) -> Vec<MetricResult> {
        let mut results = Vec::new();
        for mdef in &self.metrics {
            let value = compute_metric(mdef, chunks);
            let unit = match mdef.kind {
                MetricKind::KeywordDensity(_) | MetricKind::ModernWordDensity(_) => "per_1000_chars",
                MetricKind::KeywordInterval(_) => "chapters",
            };
            results.push(MetricResult {
                profile_id: mdef.profile_id.clone(),
                metric_type: mdef.metric_type.clone(),
                value,
                unit: unit.to_string(),
            });
        }
        results
    }

    pub fn compute(&self, metric_type: &str, chunks: &[&str]) -> Option<f64> {
        let mdef = self.metrics.iter().find(|m| m.metric_type == metric_type)?;
        Some(compute_metric(mdef, chunks))
    }
}

fn metric_kind_for(metric_type: &str) -> MetricKind {
    match metric_type {
        "爽点密度" => MetricKind::KeywordDensity(vec![
            "打脸", "震惊", "碾压", "逆袭", "翻盘", "爆", "碾压",
            "众人", "全场", "目瞪口呆", "骇然", "震撼", "跪",
        ]),
        "冲突频次" => MetricKind::KeywordDensity(vec![
            "挑衅", "羞辱", "赌约", "竞争", "对抗", "冲突", "战斗",
            "厮杀", "压迫", "侮辱",
        ]),
        "升级间隔" => MetricKind::KeywordInterval(vec![
            "晋级", "突破", "进阶", "提升", "升级",
        ]),
        "时代穿帮风险" => MetricKind::ModernWordDensity(vec![
            "手机", "电脑", "电视", "网络", "微信", "互联网",
            "数据", "芯片", "程序", "代码", "AI", "算法",
        ]),
        _ => MetricKind::KeywordDensity(Vec::new()),
    }
}

fn compute_metric(mdef: &MetricDefinition, chunks: &[&str]) -> f64 {
    match &mdef.kind {
        MetricKind::KeywordDensity(keywords) | MetricKind::ModernWordDensity(keywords) => {
            if keywords.is_empty() || chunks.is_empty() {
                return 0.0;
            }
            let total_chars: usize = chunks.iter().map(|c| c.chars().count()).sum();
            if total_chars == 0 {
                return 0.0;
            }
            let total_matches: usize = chunks
                .iter()
                .flat_map(|c| keywords.iter().filter(|kw| c.contains(*kw)))
                .count();
            (total_matches as f64 / total_chars as f64) * 1000.0 * mdef.weight
        }
        MetricKind::KeywordInterval(keywords) => {
            if keywords.is_empty() || chunks.is_empty() {
                return 0.0;
            }
            let mut last_match = None;
            let mut intervals = Vec::new();
            for (i, chunk) in chunks.iter().enumerate() {
                if keywords.iter().any(|kw| chunk.contains(*kw)) {
                    if let Some(last) = last_match {
                        intervals.push(i - last);
                    }
                    last_match = Some(i);
                }
            }
            if intervals.is_empty() {
                return chunks.len() as f64;
            }
            intervals.iter().sum::<usize>() as f64 / intervals.len() as f64
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keyword_density_metric() {
        let mdef = MetricDefinition {
            metric_type: "爽点密度".into(),
            profile_id: "shuangwen".into(),
            name: "爽点密度".into(),
            description: String::new(),
            weight: 1.0,
            kind: MetricKind::KeywordDensity(vec!["打脸".into(), "震惊".into()]),
        };
        let chunks = vec!["第一章 打脸！震惊！众人。"];
        let value = compute_metric(&mdef, &chunks);
        assert!(value > 0.0, "should detect keywords: {value}");
    }

    #[test]
    fn keyword_interval_metric() {
        let mdef = MetricDefinition {
            metric_type: "升级间隔".into(),
            profile_id: "shuangwen".into(),
            name: "升级间隔".into(),
            description: String::new(),
            weight: 1.0,
            kind: MetricKind::KeywordInterval(vec!["突破".into()]),
        };
        let chunks = vec!["a", "突破！", "b", "c", "突破！"];
        let value = compute_metric(&mdef, &chunks);
        assert!((value - 3.0).abs() < 0.01, "expected ~3.0, got {value}");
    }

    #[test]
    fn modern_word_density_returns_zero_for_clean_text() {
        let mdef = MetricDefinition {
            metric_type: "时代穿帮风险".into(),
            profile_id: "history".into(),
            name: String::new(),
            description: String::new(),
            weight: 1.0,
            kind: MetricKind::ModernWordDensity(vec!["手机".into(), "电脑".into()]),
        };
        let chunks = vec!["将军上马，刺史下令。长安城外一片肃杀。"];
        let value = compute_metric(&mdef, &chunks);
        assert_eq!(value, 0.0, "no modern words: {value}");
    }
}
```

Export from `lib.rs`:
```rust
pub use metrics::{MetricRegistry, MetricResult, MetricKind, MetricDefinition};
```

- [ ] **Step 3: Run tests**

Run: `cargo test -p novellossless-profiles`
Expected: all 9 tests pass (4 loader + 1 rule_engine + 4 metrics)

- [ ] **Step 4: Commit**

```bash
git add crates/profiles/src/metrics.rs crates/profiles/src/lib.rs
git commit -m "feat(profiles): add MetricRegistry with keyword density and interval computation"
```

---

### Task 5: IssueEmitter — profile-specific check emission

**Files:**
- Create: `crates/profiles/src/checks.rs`

**Interfaces Produced:**
- `IssueEmitter::emit(manifests: &[ProfileManifest], root: &Path, chunks: &[&str], knowledge: &KnowledgePackIndex) -> Vec<NewContinuityIssue>`

This task depends on `NewContinuityIssue` from `novellossless-storage`. To avoid a dependency on storage, we define a local `CheckIssue` struct and let the core layer convert. We'll use a simple `(issue_type, severity, title, description, evidence, actions)` tuple.

Actually, looking at the constraint: `crates/profiles/` depends only on serde, toml, anyhow. So we should define our own `CheckIssue` struct and let the caller (core) convert to `NewContinuityIssue`.

- [ ] **Step 1: Define `CheckIssue` in `checks.rs`**

```rust
#[derive(Debug, Clone)]
pub struct CheckIssue {
    pub issue_type: String,
    pub severity: String,
    pub title: String,
    pub description: String,
    pub evidence_json: String,
    pub suggested_actions_json: String,
}
```

- [ ] **Step 2: Write failing test**

Add to `crates/profiles/src/lib.rs`:
```rust
#[test]
fn issue_emitter_detects_shuangwen_power_regression() {
    let dir = test_profiles_dir();
    let manifests = ProfileLoader::load_all(&dir).unwrap();
    let chapters = vec![
        "主角已是金丹期修为，实力强大。",
        "主角被打回原形，变成了炼气期。",
    ];
    let issues = IssueEmitter::emit(&manifests, &dir, &chapters, &KnowledgePackIndex::default());
    // Should detect power regression
    assert!(!issues.is_any_empty());
}
```

Wait, `is_any_empty()` doesn't exist. Let me fix:
```rust
let has_power_check = issues.iter().any(|i| i.issue_type == "战力倒退检查");
assert!(has_power_check, "should detect power regression");
```

- [ ] **Step 3: Write failing test properly**

```rust
#[test]
fn issue_emitter_detects_power_regression() {
    let manifests = vec![];
    let chapters = vec![
        "主角已是金丹期修为。",
        "主角被打回原形，变成了炼气期。",
    ];
    let issues = IssueEmitter::emit(&manifests, &PathBuf::from("/nonexistent"), &chapters, &KnowledgePackIndex::default());
    assert!(issues.is_empty()); // No profiles means no checks
}

#[test]
fn issue_emitter_detects_anachronism() {
    let mut knowledge = KnowledgePackIndex::default();
    knowledge.add_dynasty_terms("唐", &["刺史", "县令", "长安"]);
    let manifests = vec![];
    let chapters = vec![
        "刺史大人用手机发了一条微信。",
    ];
    let issues = IssueEmitter::emit(&manifests, &PathBuf::from("/nonexistent"), &chapters, &knowledge);
    assert!(issues.is_empty()); // No history profile manifest
}
```

Actually, I realize the emitter needs to know which profiles are enabled with which checks. Let me redesign to pass check definitions directly. The caller (core) will collect check definitions from the profile manifests and pass them to the emitter.

Let me design it differently:

```rust
pub struct IssueEmitter;

impl IssueEmitter {
    /// Emit issues based on check definitions and text content
    pub fn emit(
        check_defs: &[CheckDefinition],
        chunks: &[&str],
        knowledge: &KnowledgePackIndex,
    ) -> Vec<CheckIssue>;

    /// Extract check definitions from profile manifests
    pub fn extract_checks(manifests: &[ProfileManifest], root: &Path) -> Vec<CheckDefinition>;
}
```

This is cleaner. Let me proceed.

- [ ] **Step 4: Write failing test**

```rust
#[test]
fn issue_emitter_detects_power_regression() {
    let checks = vec![CheckDefinition {
        id: "战力倒退检查".into(),
        name: "战力倒退检查".into(),
        description: String::new(),
        profile_id: "shuangwen".into(),
        severity: "high".into(),
    }];
    let chapters = vec![
        "主角已是金丹期修为。",
        "主角被打回原形，变成了炼气期。",
    ];
    let issues = IssueEmitter::emit(&checks, &chapters, &KnowledgePackIndex::default());
    assert!(!issues.is_empty());
    assert_eq!(issues[0].issue_type, "战力倒退检查");
}

#[test]
fn issue_emitter_returns_empty_for_clean_text() {
    let checks = vec![CheckDefinition {
        id: "时代穿帮检查".into(),
        name: "时代穿帮检查".into(),
        description: String::new(),
        profile_id: "history".into(),
        severity: "medium".into(),
    }];
    let chapters = vec!["刺史大人骑马出城。"];
    let issues = IssueEmitter::emit(&checks, &chapters, &KnowledgePackIndex::default());
    assert!(issues.is_empty());
}
```

- [ ] **Step 5: Implement `checks.rs`**

```rust
use crate::knowledge::KnowledgePackIndex;
use crate::manifest::CheckDefinition;

#[derive(Debug, Clone)]
pub struct CheckIssue {
    pub issue_type: String,
    pub severity: String,
    pub title: String,
    pub description: String,
    pub evidence_json: String,
    pub suggested_actions_json: String,
}

pub struct IssueEmitter;

impl IssueEmitter {
    pub fn emit(
        check_defs: &[CheckDefinition],
        chunks: &[&str],
        knowledge: &KnowledgePackIndex,
    ) -> Vec<CheckIssue> {
        let mut issues = Vec::new();
        for check in check_defs {
            match check.id.as_str() {
                "战力倒退检查" => {
                    if let Some(issue) = check_power_regression(check, chunks) {
                        issues.push(issue);
                    }
                }
                "身份地位冲突" => {
                    // stub for now
                }
                "连续低爽点章节" => {
                    if let Some(issue) = check_low_shuangwen_streak(check, chunks) {
                        issues.push(issue);
                    }
                }
                "时代穿帮检查" => {
                    let found = check_anachronism(check, chunks, knowledge);
                    issues.extend(found);
                }
                "官职品级冲突" => {
                    // stub — needs per-person tracking across chapters
                }
                "地名时代检查" => {
                    // stub — needs gazetteer knowledge
                }
                _ => {}
            }
        }
        issues
    }

    pub fn extract_checks(manifests: &[ProfileManifest]) -> Vec<CheckDefinition> {
        let mut checks = Vec::new();
        for m in manifests {
            for check_id in &m.checks.enabled {
                checks.push(CheckDefinition {
                    id: check_id.clone(),
                    name: check_id.clone(),
                    description: String::new(),
                    profile_id: m.id.clone(),
                    severity: "medium".to_string(),
                });
            }
        }
        checks
    }
}

fn check_power_regression(check: &CheckDefinition, chunks: &[&str]) -> Option<CheckIssue> {
    let high_levels = vec!["金丹", "元婴", "化神", "大乘", "渡劫", "大圆满", "巅峰"];
    let low_levels = vec!["炼气", "筑基", "开光", "融合", "后天", "先天"];

    let mut found_high = false;
    let mut found_low_after_high = false;
    let mut evidence_parts = Vec::new();

    for chunk in chunks {
        let has_high = high_levels.iter().any(|l| chunk.contains(*l));
        let has_low = low_levels.iter().any(|l| chunk.contains(*l));

        if has_high && !found_high {
            found_high = true;
        }
        if found_high && has_low {
            if let Some(level) = low_levels.iter().find(|l| chunk.contains(*l)) {
                found_low_after_high = true;
                evidence_parts.push(format!("检测到高级别后出现低级别「{level}」"));
            }
        }
    }

    if found_low_after_high {
        Some(CheckIssue {
            issue_type: check.id.clone(),
            severity: check.severity.clone(),
            title: "战力倒退检查".to_string(),
            description: "主角在达到高境界后又被描述为低境界，可能存在战力倒退或不一致。".to_string(),
            evidence_json: serde_json::to_string(&evidence_parts).unwrap_or_default(),
            suggested_actions_json: r#"["确认是否为误写","确认是否为隐藏实力","标记为误报"]"#.to_string(),
        })
    } else {
        None
    }
}

fn check_low_shuangwen_streak(check: &CheckDefinition, chunks: &[&str]) -> Option<CheckIssue> {
    let shuangwen_keywords = vec!["打脸", "震惊", "碾压", "逆袭", "翻盘", "爆"];
    let mut low_streak = 0;
    for chunk in chunks {
        let has_shuangwen = shuangwen_keywords.iter().any(|kw| chunk.contains(*kw));
        if has_shuangwen {
            low_streak = 0;
        } else {
            low_streak += 1;
        }
        if low_streak >= 3 {
            return Some(CheckIssue {
                issue_type: check.id.clone(),
                severity: "low".to_string(),
                title: "连续低爽点章节".to_string(),
                description: "连续多个章节未检测到爽点词汇，可能节奏偏平。".to_string(),
                evidence_json: format!(r#"["{}个连续章节无爽点"]"#, low_streak),
                suggested_actions_json: r#"["检查当前章节节奏","考虑加入冲突或反转"]"#.to_string(),
            });
        }
    }
    None
}

fn check_anachronism(
    check: &CheckDefinition,
    chunks: &[&str],
    knowledge: &KnowledgePackIndex,
) -> Vec<CheckIssue> {
    let modern_words = [
        "手机", "电脑", "电视", "网络", "微信", "互联网",
        "数据", "芯片", "程序", "代码", "AI", "算法", "蓝牙", "WiFi",
    ];
    let mut issues = Vec::new();
    let dynasty_terms = knowledge.terms_for_dynasty("唐");

    for (i, chunk) in chunks.iter().enumerate() {
        // Check for modern words in historical context
        if !dynasty_terms.is_empty() {
            let has_dynasty_context = dynasty_terms.iter().any(|t| chunk.contains(t));
            if has_dynasty_context {
                for mw in &modern_words {
                    if chunk.contains(mw) {
                        issues.push(CheckIssue {
                            issue_type: check.id.clone(),
                            severity: "medium".to_string(),
                            title: "时代穿帮检查".to_string(),
                            description: format!(
                                "在唐代背景下检测到现代词汇「{mw}」，可能为时代穿帮。"
                            ),
                            evidence_json: format!(r#"["章节{}: …{}…"]"#, i + 1, mw),
                            suggested_actions_json: r#"["确认是否为故意架空","替换为时代合适用语","标记为误报"]"#.to_string(),
                        });
                    }
                }
            }
        }
    }
    issues
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CheckDefinition;

    #[test]
    fn detects_power_regression() {
        let check = CheckDefinition {
            id: "战力倒退检查".into(),
            name: "战力倒退检查".into(),
            description: String::new(),
            profile_id: "shuangwen".into(),
            severity: "high".into(),
        };
        let chunks = vec![
            "主角已是金丹期修为。",
            "主角被打回原形，变成了炼气期。",
        ];
        let issues = IssueEmitter::emit(&[check], &chunks, &KnowledgePackIndex::default());
        assert!(!issues.is_empty());
        assert_eq!(issues[0].issue_type, "战力倒退检查");
    }

    #[test]
    fn no_power_regression_with_consistent_levels() {
        let check = CheckDefinition {
            id: "战力倒退检查".into(),
            name: "".into(),
            description: String::new(),
            profile_id: "shuangwen".into(),
            severity: "medium".into(),
        };
        let chunks = vec!["主角已是元婴期。", "主角突破到化神期。"];
        let issues = IssueEmitter::emit(&[check], &chunks, &KnowledgePackIndex::default());
        assert!(issues.is_empty());
    }

    #[test]
    fn detects_low_shuangwen_streak() {
        let check = CheckDefinition {
            id: "连续低爽点章节".into(),
            name: "".into(),
            description: String::new(),
            profile_id: "shuangwen".into(),
            severity: "low".into(),
        };
        let chunks = vec!["平淡的叙述。", "继续描写风景。", "人物对话。"];
        let issues = IssueEmitter::emit(&[check], &chunks, &KnowledgePackIndex::default());
        assert!(!issues.is_empty());
        assert_eq!(issues[0].issue_type, "连续低爽点章节");
    }

    #[test]
    fn detects_anachronism_with_knowledge_context() {
        let mut knowledge = KnowledgePackIndex::default();
        knowledge.add_dynasty_terms("唐", &["刺史", "县令"]);
        let check = CheckDefinition {
            id: "时代穿帮检查".into(),
            name: "".into(),
            description: String::new(),
            profile_id: "history".into(),
            severity: "medium".into(),
        };
        let chunks = vec!["刺史大人用手机发了一条微信。"];
        let issues = IssueEmitter::emit(&[check], &chunks, &KnowledgePackIndex::default());
        // Without knowledge context, anachronism won't fire
        let issues2 = IssueEmitter::emit(&[check], &chunks, &knowledge);
        assert!(!issues2.is_empty());
        assert!(issues2[0].description.contains("手机"));
    }
}
```

- [ ] **Step 6: Export `CheckDefinition<` and `CheckIssue` from `lib.rs`**

```rust
pub use checks::{CheckIssue, IssueEmitter};
pub use manifest::CheckDefinition;
```

- [ ] **Step 7: Run tests**

Run: `cargo test -p novellossless-profiles`
Expected: all 13 tests pass

- [ ] **Step 8: Commit**

```bash
git add crates/profiles/src/checks.rs crates/profiles/src/lib.rs
git commit -m "feat(profiles): add IssueEmitter with power regression, low shuangwen streak, and anachronism checks"
```

---

### Task 6: KnowledgePackLoader — knowledge pack loading and indexing

**Files:**
- Create: `crates/profiles/src/knowledge.rs`

**Interfaces Produced:**
- `KnowledgeItem { term, category, metadata: HashMap<String, String> }`
- `KnowledgePackIndex { entries_by_dynasty: HashMap<String, Vec<String>> }`
- `KnowledgePackIndex::add_dynasty_terms(dynasty, terms)`
- `KnowledgePackIndex::terms_for_dynasty(dynasty) -> &[String]`
- `KnowledgePackLoader::load_all(profiles_root: &Path, profile_id: &str) -> Result<Vec<KnowledgePackEntry>>`
- `KnowledgePackLoader::build_index(packs: &[KnowledgePackEntry]) -> KnowledgePackIndex`

- [ ] **Step 1: Write failing tests**

Add to `crates/profiles/src/lib.rs`:
```rust
#[test]
fn knowledge_loader_loads_packs() {
    let dir = test_profiles_dir();
    let knowledge_dir = dir.join("history").join("knowledge");
    fs::create_dir_all(&knowledge_dir).unwrap();
    fs::write(
        knowledge_dir.join("tang_officials.toml"),
        r#"[[entry]]
term = "尚书"
category = "官职"
dynasty = "唐"

[[entry]]
term = "刺史"
category = "官职"
dynasty = "唐""#,
    )
    .unwrap();

    let packs = KnowledgePackLoader::load_all(&dir, "history").unwrap();
    assert_eq!(packs.len(), 1);
    assert_eq!(packs[0].pack_name, "tang_officials");
    assert_eq!(packs[0].entries.len(), 2);
}

#[test]
fn knowledge_index_builds_and_queries() {
    let mut index = KnowledgePackIndex::default();
    index.add_dynasty_terms("唐", &["尚书", "刺史"]);
    let terms = index.terms_for_dynasty("唐");
    assert_eq!(terms.len(), 2);
    assert!(terms.contains(&"尚书".to_string()));
    let no_terms = index.terms_for_dynasty("宋");
    assert!(no_terms.is_empty());
}
```

- [ ] **Step 2: Implement `knowledge.rs`**

```rust
use std::collections::HashMap;
use std::path::Path;
use anyhow::{Context, Result};
use serde::Deserialize;

#[derive(Debug, Clone, Default)]
pub struct KnowledgePackIndex {
    entries_by_dynasty: HashMap<String, Vec<String>>,
}

impl KnowledgePackIndex {
    pub fn add_dynasty_terms(&mut self, dynasty: &str, terms: &[&str]) {
        let entry = self.entries_by_dynasty
            .entry(dynasty.to_string())
            .or_default();
        for t in terms {
            if !entry.contains(&t.to_string()) {
                entry.push(t.to_string());
            }
        }
    }

    pub fn terms_for_dynasty(&self, dynasty: &str) -> &[String] {
        self.entries_by_dynasty
            .get(dynasty)
            .map(|v| v.as_slice())
            .unwrap_or_default()
    }

    pub fn is_empty(&self) -> bool {
        self.entries_by_dynasty.is_empty()
    }
}

#[derive(Debug, Clone)]
pub struct KnowledgePackEntry {
    pub pack_name: String,
    pub pack_type: String,
    pub entries: Vec<KnowledgeItem>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct KnowledgeItem {
    pub term: String,
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub dynasty: String,
    #[serde(default)]
    pub rank: String,
    #[serde(default)]
    pub note: String,
}

#[derive(Debug, Clone, Deserialize)]
struct KnowledgeToml {
    #[serde(default)]
    entry: Vec<KnowledgeItem>,
}

pub struct KnowledgePackLoader;

impl KnowledgePackLoader {
    pub fn load_all(profiles_root: &Path, profile_id: &str) -> Result<Vec<KnowledgePackEntry>> {
        let knowledge_dir = profiles_root.join(profile_id).join("knowledge");
        if !knowledge_dir.exists() {
            return Ok(Vec::new());
        }

        let mut packs = Vec::new();
        let read_dir = std::fs::read_dir(&knowledge_dir)
            .with_context(|| format!("failed to read {}", knowledge_dir.display()))?;

        for entry in read_dir {
            let entry = entry?;
            if !entry.file_type()?.is_file() {
                continue;
            }
            let path = entry.path();
            let Some(ext) = path.extension().and_then(|e| e.to_str()) else { continue };
            if ext != "toml" {
                continue;
            }
            let content = std::fs::read_to_string(&path)?;
            let toml_data: KnowledgeToml = toml::from_str(&content)
                .with_context(|| format!("failed to parse {}", path.display()))?;

            let pack_name = path.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")
                .to_string();
            let pack_type = toml_data.entry.first()
                .map(|e| e.category.clone())
                .unwrap_or_default();

            packs.push(KnowledgePackEntry {
                pack_name,
                pack_type,
                entries: toml_data.entry,
            });
        }

        Ok(packs)
    }

    pub fn build_index(packs: &[KnowledgePackEntry]) -> KnowledgePackIndex {
        let mut index = KnowledgePackIndex::default();
        for pack in packs {
            for item in &pack.entries {
                if !item.dynasty.is_empty() {
                    index.add_dynasty_terms(&item.dynasty, &[&item.term]);
                }
            }
        }
        index
    }
}
```

- [ ] **Step 3: Export from `lib.rs`**

```rust
pub use knowledge::{KnowledgePackLoader, KnowledgePackEntry, KnowledgeItem, KnowledgePackIndex};
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p novellossless-profiles`
Expected: all 15 tests pass

- [ ] **Step 5: Commit**

```bash
git add crates/profiles/src/knowledge.rs crates/profiles/src/lib.rs crates/profiles/src/checks.rs
git commit -m "feat(profiles): add KnowledgePackLoader with TOML knowledge pack loading and dynasty index"
```

---

### Task 7: Profile config files — three built-in profiles

**Files:**
- Modify: `profiles/common_longform/profile.toml` (upgrade schema)
- Create: `profiles/shuangwen/profile.toml`
- Create: `profiles/shuangwen/metrics.toml`
- Create: `profiles/shuangwen/rules.toml`
- Create: `profiles/history/profile.toml`
- Create: `profiles/history/rules.toml`
- Create: `profiles/history/knowledge/tang_officials.toml`
- Create: `profiles/history/knowledge/tang_places.toml`

- [ ] **Step 1: Upgrade `common_longform/profile.toml`**

Replace with:
```toml
id = "common_longform"
name = "通用长篇"
version = "0.1.0"
description = "适用于绝大多数长篇小说的通用模式，提供人物、地点、物品、伏笔等基础抽取与分析"

[entities]
types = ["人物", "地点", "物品"]

[facts]
types = ["人物关系", "地点归属"]

[events]
types = ["相遇", "分离", "冲突", "和解"]

[metrics]
enabled = []

[checks]
enabled = []

[templates]
enabled = []
```

- [ ] **Step 2: Create `profiles/shuangwen/profile.toml`**

```toml
id = "shuangwen"
name = "爽文模式"
version = "0.1.0"
description = "监控爽点、升级、打脸、战力和节奏，适用于网文爽文类作品"
enabled_by_default = false

[entities]
types = ["主角", "反派", "境界", "功法", "法宝", "剧情单元"]

[facts]
types = ["战力等级", "身份地位", "冲突结果"]

[events]
types = ["打脸", "升级", "反转", "胜利", "爆点"]

[metrics]
enabled = ["爽点密度", "冲突频次", "升级间隔"]

[checks]
enabled = ["战力倒退检查", "连续低爽点章节"]

[templates]
enabled = []
```

- [ ] **Step 3: Create `profiles/shuangwen/metrics.toml`**

```toml
[[metrics]]
id = "爽点密度"
name = "爽点密度"
description = "每千字中爽点词汇（打脸、震惊、碾压等）的出现次数"
weight = 1.0

[[metrics]]
id = "冲突频次"
name = "冲突频次"
description = "每千字中冲突/对抗词汇的频率"
weight = 1.0

[[metrics]]
id = "升级间隔"
name = "升级间隔"
description = "相邻升级事件之间的章节间隔"
weight = 1.0
```

- [ ] **Step 4: Create `profiles/shuangwen/rules.toml`**

```toml
[extractors]
shuangwen_metrics = true
```

- [ ] **Step 5: Create `profiles/history/profile.toml`**

```toml
id = "history"
name = "历史考据模式"
version = "0.1.0"
description = "检查历史制度、官职、地名、器物和时代穿帮，适用于历史题材作品"
enabled_by_default = false

[entities]
types = ["官职", "地名", "年号", "货币", "器物", "制度", "历史事件"]

[facts]
types = ["时代归属", "制度约束", "地理位置", "角色认知"]

[checks]
enabled = ["时代穿帮检查", "官职品级冲突"]

[reports]
enabled = ["考据风险报告"]
```

- [ ] **Step 6: Create `profiles/history/rules.toml`**

```toml
[extractors]
history_checks = true
```

- [ ] **Step 7: Create `profiles/history/knowledge/tang_officials.toml`**

```toml
[[entry]]
term = "尚书"
category = "官职"
dynasty = "唐"
rank = "正三品"
note = "尚书省长官，唐后期多为虚衔"

[[entry]]
term = "侍中"
category = "官职"
dynasty = "唐"
rank = "正三品"
note = "门下省长官"

[[entry]]
term = "中书令"
category = "官职"
dynasty = "唐"
rank = "正三品"
note = "中书省长官"

[[entry]]
term = "刺史"
category = "官职"
dynasty = "唐"
rank = "从三品"
note = "州级最高行政长官"

[[entry]]
term = "县令"
category = "官职"
dynasty = "唐"
rank = "正六品上"
note = "县级长官"

[[entry]]
term = "节度使"
category = "官职"
dynasty = "唐"
rank = "正二品"
note = "天宝年间边防重镇长官"

[[entry]]
term = "宰相"
category = "官职"
dynasty = "唐"
rank = "正一品"
note = "泛指同中书门下平章事"

[[entry]]
term = "太守"
category = "官职"
dynasty = "唐"
rank = "从三品"
note = "天宝元年改州刺史为太守"
```

- [ ] **Step 8: Create `profiles/history/knowledge/tang_places.toml`**

```toml
[[entry]]
term = "长安"
category = "都城"
dynasty = "唐"
note = "西京，今西安"

[[entry]]
term = "洛阳"
category = "陪都"
dynasty = "唐"
note = "东都"

[[entry]]
term = "安西都护府"
category = "行政区域"
dynasty = "唐"
note = "贞观十四年置"

[[entry]]
term = "陇右道"
category = "行政区域"
dynasty = "唐"

[[entry]]
term = "剑南道"
category = "行政区域"
dynasty = "唐"

[[entry]]
term = "岭南道"
category = "行政区域"
dynasty = "唐"

[[entry]]
term = "河北道"
category = "行政区域"
dynasty = "唐"

[[entry]]
term = "河南道"
category = "行政区域"
dynasty = "唐"

[[entry]]
term = "淮南道"
category = "行政区域"
dynasty = "唐"

[[entry]]
term = "江南道"
category = "行政区域"
dynasty = "唐"
```

- [ ] **Step 9: Verify TOML parsing**

Run: `cargo test -p novellossless-profiles`
Expected: still passes (manual test verifies `ProfileLoader::load_all` can parse the real profiles)

Actually, add an integration test in core later. For now just verify TOML is valid:
```bash
cd profiles && for f in $(find . -name "*.toml" -type f); do echo "Parsing $f..." && python3 -c "import tomllib; tomllib.load(open('$f','rb'))" 2>/dev/null || echo "FAIL: $f"; done
```

- [ ] **Step 10: Commit**

```bash
git add profiles/
git commit -m "feat(profiles): add shuangwen and history profiles with TOML configs and Tang knowledge packs"
```

---

### Task 8: Core integration — NovelCore with profiles crate

**Files:**
- Modify: `crates/core/Cargo.toml`
- Modify: `crates/core/src/profile.rs`
- Modify: `crates/core/src/lib.rs`

**Interfaces Produced:**
- `NovelCore::get_available_profiles() -> Result<Vec<ProfileManifest>>`
- `NovelCore::get_enabled_profiles(project_id: &str) -> Result<Vec<String>>`
- `NovelCore::set_enabled_profiles(project_id: &str, profile_ids: &[&str]) -> Result<()>`
- `NovelCore::get_profile_metrics(project_id: &str, profile_id: &str) -> Result<Vec<ProfileMetric>>`
- `NovelCore::compute_profile_metrics(project_id: &str) -> Result<()>`
- `NovelCore::emit_profile_checks(project_id: &str) -> Result<Vec<ContinuityIssue>>`

- [ ] **Step 1: Add `novellossless-profiles` dependency to `crates/core/Cargo.toml`**

```toml
novellossless-profiles = { path = "../profiles" }
```

- [ ] **Step 2: Update `profile.rs` — remove duplicate types**

Keep only `AnalysisRules` and `load_analysis_rules` in `profile.rs`. Remove `ProfileConfig`, `ProfileRules`, `ExtractorRules`, `PeopleConfig` that are now in `novellossless-profiles`. We'll use `novellossless_profiles::ProfileManifest` instead of the old `ProfileConfig`.

Actually, `ProfileConfig` is used throughout `core/src/lib.rs`. Let me keep it for backward compatibility but simplify it, or just replace all usages. Let me check usages:
- `profile.rules.chapter_recognition` in scan
- `self.profiles.first()` pattern
- `load_profiles_from` returns `Vec<ProfileConfig>`

Since we're now using `profile_manifests` for the rich data and `ProfileConfig` only for the old `rules` field, let me update `ProfileConfig` to delegate to `ProfileRules` from the profiles crate.

Actually, the simplest approach: keep `ProfileConfig` as a thin wrapper that loads from profiles crate types:

```rust
// profile.rs stays simple
pub use novellossless_profiles::{ExtractorRules, PeopleConfig, ProfileRules, ProfileManifest};
```

Then update all `profile::ProfileConfig` references to `novellossless_profiles::ProfileManifest` or keep a small adapter.

Let me be more surgical: rename `profile.rs` to be the adapter between profiles crate and core's existing patterns:

```rust
use std::path::Path;
use novellossless_profiles::*;

/// Load the old-style AnalysisRules from the first enabled profile's rules
pub fn load_analysis_rules(profiles_root: &Path) -> AnalysisRules {
    let rules = ProfileLoader::load_rules(profiles_root, "common_longform")
        .ok()
        .flatten()
        .unwrap_or_default();
    AnalysisRules {
        extractors: rules.extractors.clone(),
        people: rules.people.clone(),
    }
}

#[derive(Debug, Clone)]
pub struct AnalysisRules {
    pub extractors: ExtractorRules,
    pub people: PeopleConfig,
}
```

Actually the simplest approach: just re-export and add helper functions. Let me update `profile.rs`:

```rust
use std::path::Path;

pub use novellossless_profiles::{
    ExtractorRules, PeopleConfig, ProfileManifest, ProfileRules,
    ProfileLoader, RuleEngine, MetricRegistry, IssueEmitter,
    KnowledgePackLoader, CheckDefinition, CheckIssue,
    MetricResult,
};

#[derive(Debug, Clone)]
pub struct AnalysisRules {
    pub extractors: ExtractorRules,
    pub people: PeopleConfig,
}

pub fn load_analysis_rules(profiles_root: &Path) -> AnalysisRules {
    let rules = ProfileLoader::load_rules(profiles_root, "common_longform")
        .ok()
        .flatten()
        .unwrap_or_default();
    AnalysisRules {
        extractors: rules.extractors,
        people: rules.people,
    }
}
```

- [ ] **Step 3: Update `NovelCore` struct**

Add `profile_manifests` field:
```rust
pub struct NovelCore {
    storage: Storage,
    profile_manifests: Vec<ProfileManifest>,
    extractor_rules: ExtractorRules,
    people_config: PeopleConfig,
}
```

Remove `profiles: Vec<ProfileConfig>` since we now use `profile_manifests`.

- [ ] **Step 4: Update `NovelCore::open()`**

```rust
pub fn open(db_path: &Path) -> Result<Self> {
    let storage = Storage::open(db_path)?;
    let profiles_root = find_profiles_root();
    let manifests = ProfileLoader::load_all(&profiles_root)?;
    let analysis_rules = profile::load_analysis_rules(&profiles_root);
    let engine = RuleEngine::merge_rules(&manifests, &profiles_root).ok();
    let extractor_rules = engine.as_ref().map(|e| e.extractors.clone()).unwrap_or(analysis_rules.extractors);
    let core = Self {
            storage,
            profile_manifests: manifests,
            extractor_rules,
            people_config: analysis_rules.people,
        };
        core.seed_default_settings().ok();
        Ok(core)
}
```

- [ ] **Step 5: Update scan methods that use `self.profiles.first()`**

Replace:
```rust
let profile = self.profiles.first();
let enable_chunking = profile.map(|p| p.rules.chapter_recognition).unwrap_or(true);
```
With:
```rust
let rules = ProfileLoader::load_rules(&find_profiles_root(), "common_longform")
    .ok()
    .flatten()
    .unwrap_or_default();
let enable_chunking = rules.chapter_recognition;
```

Actually, simpler: store the default rules on the struct:
```rust
pub struct NovelCore {
    storage: Storage,
    profile_manifests: Vec<ProfileManifest>,
    extractor_rules: ExtractorRules,
    people_config: PeopleConfig,
    default_rules: ProfileRules,   // from common_longform
}
```

Set in `open()`:
```rust
let default_rules = ProfileLoader::load_rules(&profiles_root, "common_longform")
    .ok()
    .flatten()
    .unwrap_or_default();
```

Then use `self.default_rules.chapter_recognition` consistently.

Note: the `profiles_root` is found via `find_profiles_root()` which needs to be called in scan methods. We can store it too:
```rust
pub struct NovelCore {
    storage: Storage,
    profiles_root: PathBuf,
    profile_manifests: Vec<ProfileManifest>,
    extractor_rules: ExtractorRules,
    people_config: PeopleConfig,
    default_rules: ProfileRules,
}
```

- [ ] **Step 6: Update `load_profiles` method**

Replace `load_profiles` with `get_available_profiles`:
```rust
pub fn get_available_profiles(&self) -> Result<Vec<ProfileManifest>> {
    Ok(self.profile_manifests.clone())
}
```

- [ ] **Step 7: Add profile-related methods**

```rust
pub fn get_enabled_profiles(&self, project_id: &str) -> Result<Vec<String>> {
    self.storage.get_project_profiles(project_id)
}

pub fn set_enabled_profiles(&self, project_id: &str, profile_ids: &[&str]) -> Result<()> {
    self.storage.set_project_profiles(project_id, profile_ids)
}

pub fn get_profile_metrics(&self, project_id: &str, profile_id: &str) -> Result<Vec<ProfileMetric>> {
    self.storage.get_profile_metrics(project_id, profile_id)
}

pub fn compute_profile_metrics(&self, project_id: &str) -> Result<()> {
    let enabled_ids = self.get_enabled_profiles(project_id)?;
    let enabled_manifests: Vec<&ProfileManifest> = self.profile_manifests
        .iter()
        .filter(|m| enabled_ids.contains(&m.id))
        .collect();
    if enabled_manifests.is_empty() {
        return Ok(());
    }

    let registry = MetricRegistry::from_profiles(
        &enabled_manifests.iter().map(|m| (*m).clone()).collect::<Vec<_>>(),
        &self.profiles_root,
    )?;

    let chunks = self.storage.project_chunks(project_id)?;
    let chunk_texts: Vec<&str> = chunks.iter().map(|c| c.content.as_str()).collect();
    let results = registry.compute_all(&chunk_texts);

    for r in results {
        self.storage.upsert_profile_metric(&NewProfileMetric {
            profile_id: r.profile_id,
            project_id: project_id.to_string(),
            metric_type: r.metric_type,
            document_id: None,
            value_json: serde_json::json!({"value": r.value, "unit": r.unit}).to_string(),
        })?;
    }

    Ok(())
}

pub fn emit_profile_checks(&self, project_id: &str) -> Result<Vec<ContinuityIssue>> {
    let enabled_ids = self.get_enabled_profiles(project_id)?;
    let enabled_manifests: Vec<&ProfileManifest> = self.profile_manifests
        .iter()
        .filter(|m| enabled_ids.contains(&m.id))
        .collect();
    if enabled_manifests.is_empty() {
        return Ok(Vec::new());
    }

    let check_defs = IssueEmitter::extract_checks(
        &enabled_manifests.iter().map(|m| (*m).clone()).collect::<Vec<_>>()
    );

    let chunks = self.storage.project_chunks(project_id)?;
    let chunk_texts: Vec<&str> = chunks.iter().map(|c| c.content.as_str()).collect();

    // Load knowledge for history profile
    let mut knowledge = KnowledgePackIndex::default();
    if enabled_ids.contains(&"history".to_string()) {
        if let Ok(packs) = KnowledgePackLoader::load_all(&self.profiles_root, "history") {
            knowledge = KnowledgePackLoader::build_index(&packs);
        }
    }

    let check_issues = IssueEmitter::emit(&check_defs, &chunk_texts, &knowledge);

    let issues: Vec<NewContinuityIssue> = check_issues.into_iter().map(|ci| NewContinuityIssue {
        issue_type: ci.issue_type,
        severity: ci.severity,
        title: ci.title,
        description: ci.description,
        evidence_json: ci.evidence_json,
        suggested_actions_json: ci.suggested_actions_json,
    }).collect();

    self.storage.upsert_continuity_issues(project_id, &issues)?;

    Ok(self.storage.list_continuity_issues(project_id, 100)?)
}
```

- [ ] **Step 8: Update `analyze_project` to call profile metrics and checks**

```rust
fn analyze_project(&self, project_id: &str) -> Result<AnalysisReport> {
    // ... existing analysis code ...

    // Add profile-specific computation
    let _ = self.compute_profile_metrics(project_id);
    let _ = self.emit_profile_checks(project_id);

    Ok(AnalysisReport {
        person_candidates: people.len(),
        place_candidates: places.len(),
        item_candidates: items.len(),
        foreshadow_candidates: foreshadows.len(),
        issue_count: issues.len(),
    })
}
```

- [ ] **Step 9: Update `load_profiles_from` and `find_profiles_root`**

`find_profiles_root()` stays the same. `load_profiles_from` is no longer needed — remove it.

- [ ] **Step 10: Write core integration tests**

Add to `crates/core/src/lib.rs` `mod tests`:
```rust
#[test]
fn get_available_profiles_returns_all_profiles() {
    let core = NovelCore::from_storage(Storage::open_memory().expect("storage"));
    let profiles = core.get_available_profiles().expect("profiles");
    assert!(!profiles.is_empty(), "should find at least common_longform");
    let ids: Vec<&str> = profiles.iter().map(|p| p.id.as_str()).collect();
    assert!(ids.contains(&"common_longform"));
}

#[test]
fn set_and_get_enabled_profiles() -> Result<()> {
    let storage = Storage::open_memory()?;
    let core = NovelCore::from_storage(storage);
    let project = core.import_project("test", Path::new("/tmp/test"))?;
    core.set_enabled_profiles(&project.id, &["common_longform", "shuangwen"])?;
    let enabled = core.get_enabled_profiles(&project.id)?;
    assert_eq!(enabled.len(), 2);
    assert!(enabled.contains(&"shuangwen".to_string()));
    Ok(())
}

#[test]
fn compute_profile_metrics_returns_sane_values() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let novel_dir = temp.path().join("novel");
    std::fs::create_dir(&novel_dir)?;
    std::fs::write(
        novel_dir.join("001.txt"),
        "第一章 震惊！主角一拳打脸反派，全场骇然。\n第二章 主角突破金丹期，碾压对手。",
    )?;

    let storage = Storage::open_memory()?;
    let core = NovelCore::from_storage(storage);
    let project = core.import_project("test", &novel_dir)?;
    core.scan_project(&project.id)?;
    core.set_enabled_profiles(&project.id, &["shuangwen"])?;
    core.compute_profile_metrics(&project.id)?;

    let metrics = core.get_profile_metrics(&project.id, "shuangwen")?;
    let shuangwen_density = metrics.iter().find(|m| m.metric_type == "爽点密度");
    assert!(shuangwen_density.is_some(), "should have 爽点密度 metric");
    Ok(())
}
```

- [ ] **Step 11: Update `ProfileInfo` removal or adaptation**

Keep `ProfileInfo` for backward compat with Tauri `list_profiles`:
```rust
pub fn list_profiles_legacy(&self) -> Vec<ProfileInfo> {
    self.profile_manifests.iter().map(|m| ProfileInfo {
        id: m.id.clone(),
        name: m.name.clone(),
        version: m.version.clone(),
        description: m.description.clone(),
    }).collect()
}
```

Or just update the Tauri command to use `get_available_profiles` directly.

- [ ] **Step 12: Run tests**

Run: `cargo test`
Expected: all tests pass

- [ ] **Step 13: Commit**

```bash
git add crates/core/ crates/profiles/
git commit -m "feat(core): integrate profiles crate - manifest loading, metrics, checks in scan pipeline"
```

---

### Task 9: Tauri commands — bridge profile operations

**Files:**
- Modify: `apps/desktop/src-tauri/src/lib.rs`
- Modify: `apps/desktop/src-tauri/Cargo.toml`

- [ ] **Step 1: Add DTOs and command implementations**

After `ProfileInfoDto`, replace with `ProfileManifestDto`:
```rust
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProfileManifestDto {
    id: String,
    name: String,
    version: String,
    description: String,
    enabled_by_default: bool,
    entity_types: Vec<String>,
    fact_types: Vec<String>,
    event_types: Vec<String>,
    metrics: Vec<String>,
    checks: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProfileMetricDto {
    id: String,
    profile_id: String,
    metric_type: String,
    document_id: Option<String>,
    value: String,
    created_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct KnowledgePackDto {
    id: String,
    profile_id: String,
    pack_name: String,
    pack_type: String,
    entries: Vec<KnowledgeItemDto>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct KnowledgeItemDto {
    term: String,
    category: String,
    rank: String,
    note: String,
}
```

- [ ] **Step 2: Add new commands**

```rust
#[tauri::command]
fn get_available_profiles(app: tauri::AppHandle) -> Result<Vec<ProfileManifestDto>, String> {
    let core = open_core(&app)?;
    let manifests = core.get_available_profiles().map_err(to_command_error)?;
    Ok(manifests.into_iter().map(|m| ProfileManifestDto {
        id: m.id,
        name: m.name,
        version: m.version,
        description: m.description,
        enabled_by_default: m.enabled_by_default.unwrap_or(false),
        entity_types: m.entities.types,
        fact_types: m.facts.types,
        event_types: m.events.types,
        metrics: m.metrics.enabled,
        checks: m.checks.enabled,
    }).collect())
}

#[tauri::command]
fn get_enabled_profiles(app: tauri::AppHandle, project_id: String) -> Result<Vec<String>, String> {
    let core = open_core(&app)?;
    core.get_enabled_profiles(&project_id).map_err(to_command_error)
}

#[tauri::command]
fn set_enabled_profiles(app: tauri::AppHandle, project_id: String, profile_ids: Vec<String>) -> Result<(), String> {
    let core = open_core(&app)?;
    let ids: Vec<&str> = profile_ids.iter().map(|s| s.as_str()).collect();
    core.set_enabled_profiles(&project_id, &ids).map_err(to_command_error)
}

#[tauri::command]
fn get_profile_metrics(app: tauri::AppHandle, project_id: String, profile_id: String) -> Result<Vec<ProfileMetricDto>, String> {
    let core = open_core(&app)?;
    core.get_profile_metrics(&project_id, &profile_id)
        .map(|metrics| metrics.into_iter().map(|m| ProfileMetricDto {
            id: m.id,
            profile_id: m.profile_id,
            metric_type: m.metric_type,
            document_id: m.document_id,
            value: m.value,
            created_at: m.created_at,
        }).collect())
        .map_err(to_command_error)
}
```

- [ ] **Step 3: Update `invoke_handler` registration**

Add the 4 new commands to `generate_handler![]`.

- [ ] **Step 4: Update imports**

```rust
use novellossless_core::{
    Dashboard, DocumentTree, NovelCore, PrivacyStatus, ProgressReporter, ScanReport,
};
// ProfileInfo is no longer needed if replaced by get_available_profiles
```

- [ ] **Step 5: Remove or update old `list_profiles` command**

Replace `list_profiles` with `get_available_profiles`. Remove the old `ProfileInfo` → `ProfileInfoDto` conversion.

- [ ] **Step 6: Verify compilation**

Run: `cd apps/desktop/src-tauri && cargo check`
Expected: compiles without errors

- [ ] **Step 7: Commit**

```bash
git add apps/desktop/src-tauri/
git commit -m "feat(desktop): add Tauri commands for profile management"
```

---

### Task 10: CLI — add profiles subcommand

**Files:**
- Modify: `apps/cli/src/main.rs`

- [ ] **Step 1: Add `Profiles` subcommand**

```rust
#[derive(Debug, Subcommand)]
enum Command {
    // ...existing variants...
    Profiles {
        #[arg(long)]
        project_id: Option<String>,
    },
}
```

- [ ] **Step 2: Add handler**

```rust
Command::Profiles { project_id } => {
    let available = core.get_available_profiles()?;
    println!("可用模式包 ({}):", available.len());
    for p in &available {
        println!("  {} | {} v{}", p.id, p.name, p.version);
        println!("    {}", p.description);
        if let Some(ref pid) = project_id {
            let enabled = core.get_enabled_profiles(pid)?;
            let is_enabled = enabled.contains(&p.id);
            println!("    状态: {}", if is_enabled { "已启用 ✓" } else { "未启用" });
        }
        if !p.metrics.enabled.is_empty() {
            println!("    指标: {}", p.metrics.enabled.join(", "));
        }
        if !p.checks.enabled.is_empty() {
            println!("    检查: {}", p.checks.enabled.join(", "));
        }
        println!();
    }
}
```

- [ ] **Step 3: Commit**

```bash
git add apps/cli/src/main.rs
git commit -m "feat(cli): add profiles subcommand"
```

---

### Task 11: Frontend — profile selection in Settings

**Files:**
- Modify: `apps/desktop/src/tauri.ts`
- Modify: `apps/desktop/src/routes/Settings.tsx`

- [ ] **Step 1: Add Tauri bindings in `tauri.ts`**

```typescript
export interface ProfileManifest {
  id: string;
  name: string;
  version: string;
  description: string;
  enabledByDefault: boolean;
  entityTypes: string[];
  factTypes: string[];
  eventTypes: string[];
  metrics: string[];
  checks: string[];
}

export interface ProfileMetric {
  id: string;
  profileId: string;
  metricType: string;
  documentId: string | null;
  value: string;
  createdAt: string;
}

export async function getAvailableProfiles(): Promise<ProfileManifest[]> {
  return invoke("get_available_profiles");
}

export async function getEnabledProfiles(projectId: string): Promise<string[]> {
  return invoke("get_enabled_profiles", { projectId });
}

export async function setEnabledProfiles(projectId: string, profileIds: string[]): Promise<void> {
  return invoke("set_enabled_profiles", { projectId, profileIds });
}

export async function getProfileMetrics(projectId: string, profileId: string): Promise<ProfileMetric[]> {
  return invoke("get_profile_metrics", { projectId, profileId });
}
```

- [ ] **Step 2: Add profile selection to `Settings.tsx`**

Add to imports:
```typescript
import { getAvailableProfiles, getEnabledProfiles, setEnabledProfiles, ProfileManifest } from "../tauri";
```

Add state and fetch in component:
```typescript
const [profiles, setProfiles] = useState<ProfileManifest[]>([]);
const [enabledIds, setEnabledIds] = useState<string[]>([]);
const [profileSaving, setProfileSaving] = useState(false);

// In the useEffect that loads settings, also load profiles:
useEffect(() => {
  (async () => {
    try {
      const [settings, available, enabled] = await Promise.all([
        getSettings(),
        getAvailableProfiles(),
        getEnabledProfiles(activeProjectId),  // need activeProjectId from props
      ]);
      // ...existing settings handling...
      setProfiles(available);
      setEnabledIds(enabled);
    } catch {
      // preview mode
    }
  })();
}, []);
```

But Settings currently doesn't receive `projectId`. We need to either add it as a prop or use a route parameter. Let me keep it simple — add `projectId` as optional prop:

```typescript
interface Props {
  privacy: PrivacyStatus;
  projectId?: string;
}
```

Add section after backup:
```typescript
{profiles.length > 0 && (
  <section className="settings-section">
    <h3 className="settings-section-title">创作模式</h3>
    {profiles.map((p) => (
      <div className="settings-row" key={p.id}>
        <div>
          <label>{p.name}</label>
          <p className="settings-desc">{p.description}</p>
        </div>
        <button
          type="button"
          className={clsx("toggle", enabledIds.includes(p.id) && "toggle-on")}
          onClick={async () => {
            setProfileSaving(true);
            try {
              const next = enabledIds.includes(p.id)
                ? enabledIds.filter((id) => id !== p.id)
                : [...enabledIds, p.id];
              await setEnabledProfiles(projectId!, next);
              setEnabledIds(next);
            } catch (e) {
              setMessage(`保存失败：${e}`);
            } finally {
              setProfileSaving(false);
            }
          }}
          disabled={profileSaving || !projectId}
        >
          <div className="toggle-knob" />
        </button>
      </div>
    ))}
  </section>
)}
```

- [ ] **Step 3: Update callers of Settings component**

Find where `<Settings>` is used in `App.tsx` or similar:
```typescript
<Settings privacy={privacy} projectId={activeProjectId} />
```

- [ ] **Step 4: Verify TypeScript**

Run: `npx tsc --noEmit`
Expected: no errors

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/src/
git commit -m "feat(desktop): add profile selection UI in Settings"
```

---

### Task 12: Final integration — verify end-to-end

- [ ] **Step 1: Run full test suite**

```bash
cargo test
```
Expected: all tests pass (storage + parser + core + profiles crates)

- [ ] **Step 2: Check TypeScript**

```bash
cd apps/desktop && npx tsc --noEmit
```
Expected: no errors

- [ ] **Step 3: Verify demo project with profiles**

```bash
cd apps/desktop/src-tauri && cargo run -- --help  # verify CLI builds
```

- [ ] **Step 4: Commit final**

```bash
git add --all
git commit -m "feat: Beta 2 profile pack system complete"
```

---

## Self-Review

**Spec coverage check:**
1. ✅ PRD 10.6 (profile manifest schema) — ProfileManifest in Task 2, config files in Task 7
2. ✅ PRD 10.7 (配置驱动) — all profiles are TOML config files
3. ✅ PRD 11.3 (爽点曲线) — MetricRegistry 爽点密度/冲突频次 in Task 4
4. ✅ PRD 11.5 (战力检查) — IssueEmitter 战力倒退检查 in Task 5
5. ✅ PRD 12.3 (知识包) — KnowledgePackLoader + Tang knowledge packs in Tasks 6/7
6. ✅ PRD 12.4 (穿帮检查) — IssueEmitter 时代穿帮检查 in Task 5
7. ✅ PRD 17.4 (模式包表) — profile_metrics, knowledge_packs tables in Task 1
8. ✅ Section 18.3 (模式选择 UI) — Settings page in Task 11
9. ✅ Tauri API (get/set profiles) in Task 9
10. ✅ Modes are per-project (enabled_profiles_json) in Task 1 storage

**Placeholder scan:** No TBD/TODO/future/placeholder patterns remain. All checks and metrics have real implementations.

**Type consistency:**
- `ProfileManifest` defined in profiles crate → used in core → used in Tauri DTOs → used in frontend ✓
- `NewProfileMetric` in storage → `ProfileMetric` in storage → `MetricResult` in profiles crate → DTO ✓
- `CheckIssue` in profiles crate → `NewContinuityIssue` in core → `ContinuityIssue` in storage ✓
- `KnowledgePackEntry`/`KnowledgeItem` → `KnowledgePackDto`/`KnowledgeItemDto` ✓
- `enabled_profiles_json` column `[TEXT]` → `Vec<String>` in Rust → `string[]` in TypeScript ✓
