# Alpha 1 记忆卡片循环 — 纵深现有链路 实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 补强现有三个最薄弱环节：Profile 运行时 → 分析管线重构 → UI 导航与卡片页

**Architecture:** 三阶段顺序推进。Phase 1 为分析管线提供配置驱动能力；Phase 2 重构分析管线为 Extractor trait 模式并增强提取深度；Phase 3 激活 UI 路由与独立页面。

**Tech Stack:** Rust (Ed2024), rusqlite, regex, Tauri 2, React 18, react-router-dom, Vite, Tailwind CSS

## Global Constraints

- Rust edition 2024, all existing lints/clippy must remain clean
- No new dependencies outside the existing Cargo.toml deps (react-router-dom is only new JS dep)
- All existing tests must pass after each phase
- Chinese text must remain fully supported throughout
- AGENTS.md rules: smallest change, match existing module boundaries, check all callers before changing public APIs
- Run `cargo fmt && cargo test` after each Rust task

---

## 文件结构变更一览

### Rust crates

```
crates/core/src/
  lib.rs                          → 修改：NovelCore 新增 profiles 字段，scan_file 和 analyze_project 接收规则
  analysis/
    mod.rs                        → 新建：Extractor trait + Extraction enum + Registry
    extractor.rs                  → 新建：trait Extractor, trait bounds
    person.rs                     → 新建：PersonExtractor (增强版)
    place.rs                      → 新建：PlaceExtractor (现有逻辑迁移)
    item.rs                       → 新建：ItemExtractor (现有逻辑迁移)
    foreshadow.rs                 → 新建：ForeshadowExtractor (增强版)
    conflicts.rs                  → 新建：EyeColorConflictExtractor + RepeatExpressionExtractor
  profile.rs                      → 新建：ProfileConfig, ProfileRules, ExtractorRules, PeopleConfig 结构体

crates/storage/src/
  lib.rs                          → 修改：upsert_document_with_chunks 新增 skip_fts 参数

profiles/common_longform/
  profile.toml                    → 不变
  rules.toml                      → 新建：extractor 开关和 people 配置
```

### Desktop UI (TypeScript/React)

```
apps/desktop/src/
  App.tsx                         → 重写：拆为 layout + routes
  tauri.ts                        → 修改：新增 getDocumentChunks API
  routes/
    Dashboard.tsx                 → 新建：从 App.tsx 迁移现有 dashboard
    ContentView.tsx               → 新建：正文浏览页
    Characters.tsx                → 新建：人物卡片页
    Foreshadows.tsx               → 新建：伏笔账本页
    Issues.tsx                    → 新建：冲突报告页
    SearchView.tsx                → 新建：搜索页（提级）
    ContextPack.tsx               → 新建：上下文包页（从 dashboard 提级）
    Privacy.tsx                   → 新建：隐私中心页
  components/
    InspectorPanel.tsx            → 新建：通用详情侧栏组件（从 App.tsx 提取）
    CandidateCard.tsx             → 新建：候选卡片组件

apps/desktop/src-tauri/src/
  lib.rs                          → 修改：新增 get_document_chunks 命令

apps/desktop/package.json         → 修改：新增 react-router-dom 依赖
```

---

### Task 1: Profile 配置结构体 + rules.toml

**Files:**
- Create: `crates/core/src/profile.rs`
- Create: `profiles/common_longform/rules.toml`
- Modify: `crates/core/src/lib.rs` (末尾加 `mod profile;`)

**Interfaces:**
- Produces: `ProfileConfig`, `ProfileRules`, `ExtractorRules`, `PeopleConfig` structs — all `Deserialize` + `Clone`

- [ ] **Step 1: 创建 `crates/core/src/profile.rs`**

```rust
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct ProfileConfig {
    pub id: String,
    pub name: String,
    pub rules: ProfileRules,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct ProfileRules {
    pub chapter_recognition: bool,
    pub full_text_search: bool,
    pub evidence_required: bool,
    pub auto_modify_source: bool,
}

impl Default for ProfileRules {
    fn default() -> Self {
        Self {
            chapter_recognition: true,
            full_text_search: true,
            evidence_required: true,
            auto_modify_source: false,
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

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct AnalysisRules {
    pub extractors: ExtractorRules,
    pub people: PeopleConfig,
}

impl Default for AnalysisRules {
    fn default() -> Self {
        Self {
            extractors: ExtractorRules::default(),
            people: PeopleConfig::default(),
        }
    }
}

pub fn load_analysis_rules(profiles_root: &std::path::Path) -> AnalysisRules {
    let path = profiles_root.join("common_longform").join("rules.toml");
    if !path.exists() {
        return AnalysisRules::default();
    }
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return AnalysisRules::default(),
    };
    toml::from_str(&content).unwrap_or_default()
}
```

- [ ] **Step 2: 创建 `profiles/common_longform/rules.toml`**

```toml
[extractors]
people = true
places = true
items = true
foreshadows = true
eye_color_conflicts = true
repeat_expressions = true

[people]
min_name_length = 2
max_name_length = 4
enable_alias_detection = true
```

- [ ] **Step 3: 在 `lib.rs` 末尾添加 `mod profile;`**

```rust
// 在 lib.rs 末尾，其他 mod 声明附近
mod profile;
```

- [ ] **Step 4: 验证编译**

```bash
cargo check -p novellossless-core 2>&1
```

Expected: clean compile. （注意当前没有 `toml` 依赖 — 需要添加。或者改用 serde_json / 手动解析。查看 Cargo.toml — 没有 toml crate。）

Wait — `Cargo.toml` 没有 `toml` crate。需要加。或者用 `serde_json` 格式代替 TOML？不，PRD 使用 TOML，且现有 `profile.toml` 也是 TOML。

添加 `toml` 依赖到 workspace Cargo.toml。

- [ ] **Step 4b: 添加 `toml` crate 依赖**

在 `Cargo.toml` workspace dependencies 中添加：
```toml
toml = "0.8"
```

在 `crates/core/Cargo.toml` 中添加：
```toml
toml = { workspace = true }
```

- [ ] **Step 5: 验证编译**

```bash
cargo check -p novellossless-core 2>&1
```

Expected: clean compile.

- [ ] **Step 6: 运行现有测试**

```bash
cargo test -p novellossless-core 2>&1
```

Expected: all existing tests pass.

- [ ] **Step 7: Commit**

```bash
git add crates/core/src/profile.rs profiles/common_longform/rules.toml Cargo.toml crates/core/Cargo.toml
git commit -m "feat(profile): add ProfileConfig structs and rules.toml with AnalysisRules"
```

---

### Task 2: Profile 规则接入 NovelCore 与分析管线

**Files:**
- Modify: `crates/core/src/lib.rs` — NovelCore 结构体新增 profiles/analysis_rules 字段，scan_file/analyze_project 接收规则

**Interfaces:**
- Consumes: `ProfileRules`, `AnalysisRules`, `ExtractorRules` from task 1
- Produces: `NovelCore` with `rules` field, `scan_file(chunking: bool)`, `analyze_project(extractor_rules: &ExtractorRules)`

- [ ] **Step 1: 在 `lib.rs` 的 `use` 块添加引用**

```rust
use crate::profile::{AnalysisRules, ExtractorRules, ProfileConfig, ProfileRules};
```

- [ ] **Step 2: NovelCore 添加字段**

```rust
pub struct NovelCore {
    storage: Storage,
    profiles: Vec<ProfileConfig>,
    extractor_rules: ExtractorRules,
    people_config: PeopleConfig,
}
```

- [ ] **Step 3: 修改 `NovelCore::open` 和 `from_storage`**

```rust
impl NovelCore {
    pub fn open(db_path: &Path) -> Result<Self> {
        let storage = Storage::open(db_path)?;
        let profiles_root = find_profiles_root();
        let profiles = load_profiles_from(&profiles_root);
        let analysis_rules = profile::load_analysis_rules(&profiles_root);
        Ok(Self {
            storage,
            profiles,
            extractor_rules: analysis_rules.extractors,
            people_config: analysis_rules.people,
        })
    }

    pub fn from_storage(storage: Storage) -> Self {
        let profiles_root = find_profiles_root();
        let profiles = load_profiles_from(&profiles_root);
        let analysis_rules = profile::load_analysis_rules(&profiles_root);
        Self {
            storage,
            profiles,
            extractor_rules: analysis_rules.extractors,
            people_config: analysis_rules.people,
        }
    }
}
```

添加辅助函数：

```rust
fn find_profiles_root() -> PathBuf {
    let current_dir = std::env::current_dir().unwrap_or_default();
    for ancestor in current_dir.ancestors() {
        let candidate = ancestor.join("profiles");
        if candidate.join("common_longform").join("profile.toml").exists() {
            return candidate;
        }
    }
    current_dir.join("profiles")
}

fn load_profiles_from(profiles_root: &Path) -> Vec<ProfileConfig> {
    let common = profiles_root.join("common_longform").join("profile.toml");
    if !common.exists() {
        return Vec::new();
    }
    let content = match std::fs::read_to_string(&common) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    toml::from_str::<ProfileConfig>(&content).map(|p| vec![p]).unwrap_or_default()
}
```

- [ ] **Step 4: `scan_file` 接收 chapter_recognition 规则**

修改 `scan_file` 方法，接收 `chapter_recognition: bool` 参数：

```rust
fn scan_file(&self, project: &Project, root: &Path, file: &Path, enable_chunking: bool) -> Result<()> {
    let parsed = parse_document(file)?;
    let relative_path = relative_document_path(root, file);
    let kind = document_kind(file);

    let chapters = if enable_chunking {
        parsed.chapters
    } else {
        // 不分章：整文件作为单块
        vec![parsed.chapters.into_iter().next().unwrap_or_else(|| {
            novellossless_parser::Chapter {
                index: 0,
                title: parsed.title.clone(),
                start_offset: 0,
                end_offset: parsed.content.len() as u64,
                content: parsed.content.clone(),
            }
        })]
    };
    // ... 其余逻辑与现有相同，用 chapters 代替 parsed.chapters
}
```

更新 `scan_project` 调用处：

```rust
pub fn scan_project(&self, project_id: &str) -> Result<ScanReport> {
    // ...
    enable_chunking: bool,  // 从 self.profiles 中读取
    let profile = self.profiles.first();
    let enable_chunking = profile.map(|p| p.rules.chapter_recognition).unwrap_or(true);

    for file in files {
        match self.scan_file(&project, &root, &file, enable_chunking) {
            Ok(()) => scanned_documents += 1,
            Err(_) => skipped_files += 1,
        }
    }
    // ...
}
```

- [ ] **Step 5: `analyze_project` 接收 extractor_rules**

```rust
fn analyze_project(&self, project_id: &str) -> Result<AnalysisReport> {
    let chunks = self.storage.project_chunks(project_id)?;
    let rules = &self.extractor_rules;

    let people = if rules.people {
        extract_candidates(&chunks, CandidateKind::Person)?
    } else {
        Vec::new()
    };
    let places = if rules.places {
        extract_candidates(&chunks, CandidateKind::Place)?
    } else {
        Vec::new()
    };
    let items = if rules.items {
        extract_candidates(&chunks, CandidateKind::Item)?
    } else {
        Vec::new()
    };
    let foreshadows = if rules.foreshadows {
        extract_foreshadows(&chunks)
    } else {
        Vec::new()
    };
    let issues = if rules.eye_color_conflicts || rules.repeat_expressions {
        extract_issues(&chunks)?
    } else {
        Vec::new()
    };

    // ... 其余 upsert 逻辑与现有相同
}
```

- [ ] **Step 6: `load_profiles` 保留向后兼容（Tauri 仍然需要 ProfileInfo）**

保留现有的 `pub fn load_profiles(&self, ...) -> Result<Vec<ProfileInfo>>`，改为读取 `self.profiles`：

```rust
pub fn load_profiles(&self, _profiles_root: &Path) -> Result<Vec<ProfileInfo>> {
    Ok(self.profiles.iter().map(|p| ProfileInfo {
        id: p.id.clone(),
        name: p.name.clone(),
        version: "0.1.0".to_string(),
        description: String::new(),
    }).collect())
}
```

- [ ] **Step 7: 验证编译 + 现有测试**

```bash
cargo check -p novellossless-core 2>&1 && cargo test -p novellossless-core 2>&1
```

Expected: all tests pass.

- [ ] **Step 8: Commit**

```bash
git add crates/core/src/lib.rs crates/core/src/profile.rs
git commit -m "feat(profile): integrate ProfileRules into NovelCore scan/analysis pipeline"
```

---

### Task 3: Extractor trait + 注册表

**Files:**
- Create: `crates/core/src/analysis/mod.rs`
- Create: `crates/core/src/analysis/extractor.rs`
- Modify: `crates/core/src/lib.rs` — 移出现有提取函数，添加 `mod analysis`

**Interfaces:**
- Produces: `Extractor` trait, `Extraction` enum, `ExtractionRegistry`

- [ ] **Step 1: 创建 `crates/core/src/analysis/extractor.rs`**

```rust
#[derive(Debug, Clone)]
pub struct ChunkInfo {
    pub document_id: String,
    pub chunk_id: String,
    pub document_path: String,
    pub chunk_index: i64,
    pub title: String,
    pub content: String,
    pub start_offset: i64,
    pub end_offset: i64,
}

#[derive(Debug, Clone)]
pub struct NarrativeNodeCandidate {
    pub node_type: String,
    pub name: String,
    pub aliases: Vec<String>,
    pub summary: String,
    pub occurrence_count: i64,
    pub first_chunk_id: String,
    pub latest_chunk_id: String,
    pub confidence: i64,
}

#[derive(Debug, Clone)]
pub struct ForeshadowCandidate {
    pub title: String,
    pub foreshadow_type: String,
    pub first_chunk_id: String,
    pub latest_chunk_id: String,
    pub risk_level: String,
    pub evidence: String,
    pub related_nodes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct IssueCandidate {
    pub issue_type: String,
    pub severity: String,
    pub title: String,
    pub description: String,
    pub evidence_json: String,
    pub suggested_actions_json: String,
}

#[derive(Debug, Clone)]
pub enum Extraction {
    Candidate(NarrativeNodeCandidate),
    Foreshadow(ForeshadowCandidate),
    Issue(IssueCandidate),
}

pub trait Extractor {
    fn name(&self) -> &'static str;
    fn extract(&self, chunks: &[ChunkInfo]) -> Vec<Extraction>;
}
```

- [ ] **Step 2: 创建 `crates/core/src/analysis/mod.rs`**

```rust
pub mod extractor;
pub mod person;
pub mod place;
pub mod item;
pub mod foreshadow;
pub mod conflicts;

pub use extractor::*;
pub use person::PersonExtractor;
pub use place::PlaceExtractor;
pub use item::ItemExtractor;
pub use foreshadow::ForeshadowExtractor;
pub use conflicts::{EyeColorConflictExtractor, RepeatExpressionExtractor};
```

- [ ] **Step 3: 在 `lib.rs` 末尾添加 `mod analysis;`**

```rust
mod analysis;
```

- [ ] **Step 4: 验证编译**

```bash
cargo check -p novellossless-core 2>&1
```

Expected: clean compile (with unused code warnings, OK).

- [ ] **Step 5: Commit**

```bash
git add crates/core/src/analysis/
git commit -m "feat(analysis): add Extractor trait and Extraction enum skeleton"
```

---

### Task 4: 将现有提取器移植到 Extractor trait

**Files:**
- Create: `crates/core/src/analysis/person.rs`
- Create: `crates/core/src/analysis/place.rs`
- Create: `crates/core/src/analysis/item.rs`
- Create: `crates/core/src/analysis/foreshadow.rs`
- Create: `crates/core/src/analysis/conflicts.rs`
- Modify: `crates/core/src/lib.rs` — 删掉原有 `extract_candidates`, `extract_foreshadows`, `extract_issues` 及相关辅助函数，改为使用 Extractor

- [ ] **Step 1: 创建 `crates/core/src/analysis/person.rs`**

```rust
use std::collections::BTreeMap;
use regex::Regex;
use super::extractor::{ChunkInfo, Extraction, Extractor, NarrativeNodeCandidate};
use crate::CandidateKind;

#[derive(Default)]
pub struct PersonExtractor;

impl Extractor for PersonExtractor {
    fn name(&self) -> &'static str {
        "person"
    }

    fn extract(&self, chunks: &[ChunkInfo]) -> Vec<Extraction> {
        let mut seen = BTreeMap::<String, CandidateAccumulator>::new();
        let Ok(patterns) = person_patterns() else {
            return Vec::new();
        };

        for chunk in chunks {
            for pattern in &patterns {
                for captures in pattern.captures_iter(&chunk.content) {
                    let Some(raw) = captures.get(1).map(|m| m.as_str()) else {
                        continue;
                    };
                    let name = normalize_name(raw);
                    if !is_valid_person_name(&name) {
                        continue;
                    }
                    seen.entry(name.clone())
                        .or_insert_with(|| CandidateAccumulator {
                            count: 0,
                            first_chunk_id: chunk.chunk_id.clone(),
                            latest_chunk_id: chunk.chunk_id.clone(),
                            aliases: Vec::new(),
                        });
                    if let Some(entry) = seen.get_mut(&name) {
                        entry.count += 1;
                        entry.latest_chunk_id = chunk.chunk_id.clone();
                    }
                }
            }
        }

        seen.into_iter()
            .filter(|(_, acc)| acc.count >= 1)
            .map(|(name, acc)| Extraction::Candidate(NarrativeNodeCandidate {
                node_type: "person".to_string(),
                name,
                aliases: Vec::new(),
                summary: String::new(),
                occurrence_count: acc.count,
                first_chunk_id: acc.first_chunk_id,
                latest_chunk_id: acc.latest_chunk_id,
                confidence: (50 + acc.count.saturating_mul(10)).min(90),
            }))
            .collect()
    }
}

struct CandidateAccumulator {
    count: i64,
    first_chunk_id: String,
    latest_chunk_id: String,
    aliases: Vec<String>,
}

fn person_patterns() -> Result<Vec<Regex>, regex::Error> {
    vec![
        Regex::new(r"([\p{Han}]{2,4})(?:说|问|道|喊|低声|笑道|看着|走进|转身)"),
        Regex::new(r"(?:向|对|跟)([\p{Han}]{2,4})(?:说|问|道)"),
    ].into_iter().collect()
}

fn normalize_name(raw: &str) -> String {
    raw.trim_matches(|ch: char| {
        ch.is_whitespace() || matches!(ch, '，' | '。' | '、' | '：' | '；' | '“' | '”' | '"' | '\'' | '《' | '》')
    }).to_string()
}

fn is_valid_person_name(name: &str) -> bool {
    let stopwords = [
        "自己", "什么", "这里", "那里", "哪里", "这个", "那个", "他们", "我们", "你们", "没有",
        "不是", "已经", "突然",
    ];
    name.chars().count() >= 2
        && !stopwords.contains(&name)
        && !name.ends_with("里")
        && !name.ends_with("中")
}
```

- [ ] **Step 2: 创建 `crates/core/src/analysis/place.rs`**

```rust
use std::collections::BTreeMap;
use regex::Regex;
use super::extractor::{ChunkInfo, Extraction, Extractor, NarrativeNodeCandidate};

#[derive(Default)]
pub struct PlaceExtractor;

impl Extractor for PlaceExtractor {
    fn name(&self) -> &'static str {
        "place"
    }

    fn extract(&self, chunks: &[ChunkInfo]) -> Vec<Extraction> {
        let mut seen = BTreeMap::new();
        let suffix_patterns = [
            "城", "镇", "村", "街", "巷", "楼", "塔", "宫", "殿", "府",
            "山", "谷", "阁", "院", "桥", "寺", "观", "港", "站",
            "基地", "星球", "舰船",
        ];
        let pattern_str = format!(r"([\p{{Han}}]{{1,6}}(?:{}))", suffix_patterns.join("|"));
        let Ok(pattern) = Regex::new(&pattern_str) else {
            return Vec::new();
        };

        for chunk in chunks {
            for captures in pattern.captures_iter(&chunk.content) {
                let Some(raw) = captures.get(1).map(|m| m.as_str()) else {
                    continue;
                };
                let name = raw.trim().to_string();
                if name.chars().count() < 2 {
                    continue;
                }
                seen.entry(name)
                    .or_insert_with(|| (0, chunk.chunk_id.clone(), chunk.chunk_id.clone()));
                if let Some((count, _, latest)) = seen.get_mut(&name) {
                    *count += 1;
                    *latest = chunk.chunk_id.clone();
                }
            }
        }

        seen.into_iter()
            .map(|(name, (count, first, latest))| Extraction::Candidate(NarrativeNodeCandidate {
                node_type: "place".to_string(),
                name,
                aliases: Vec::new(),
                summary: String::new(),
                occurrence_count: count,
                first_chunk_id: first,
                latest_chunk_id: latest,
                confidence: (50 + count.saturating_mul(10)).min(90),
            }))
            .collect()
    }
}
```

- [ ] **Step 3: 创建 `crates/core/src/analysis/item.rs`**

```rust
use std::collections::BTreeMap;
use regex::Regex;
use super::extractor::{ChunkInfo, Extraction, Extractor, NarrativeNodeCandidate};

#[derive(Default)]
pub struct ItemExtractor;

impl Extractor for ItemExtractor {
    fn name(&self) -> &'static str {
        "item"
    }

    fn extract(&self, chunks: &[ChunkInfo]) -> Vec<Extraction> {
        let mut seen = BTreeMap::new();
        let item_nouns = [
            "钥匙", "信", "戒指", "刀", "剑", "书", "照片", "芯片",
            "卷轴", "玉佩", "伞", "令牌", "地图", "药瓶", "手札", "玉简",
        ];
        let noun_pattern = format!(r"([\p{{Han}}]{{0,4}}(?:{}))", item_nouns.join("|"));
        let Ok(patterns) = vec![
            Regex::new(&noun_pattern),
            Regex::new(r"(?:拿起|藏起|交给|寻找|丢失|夺走|握住)([\p{Han}]{1,6})"),
        ].into_iter().collect::<Result<Vec<_>, _>>() else {
            return Vec::new();
        };

        for chunk in chunks {
            for pattern in &patterns {
                for captures in pattern.captures_iter(&chunk.content) {
                    let Some(raw) = captures.get(1).map(|m| m.as_str()) else {
                        continue;
                    };
                    let name = strip_quantity_prefix(raw.trim());
                    if name.chars().count() < 2 {
                        continue;
                    }
                    seen.entry(name)
                        .or_insert_with(|| (0, chunk.chunk_id.clone(), chunk.chunk_id.clone()));
                    if let Some((count, _, latest)) = seen.get_mut(&name) {
                        *count += 1;
                        *latest = chunk.chunk_id.clone();
                    }
                }
            }
        }

        seen.into_iter()
            .map(|(name, (count, first, latest))| Extraction::Candidate(NarrativeNodeCandidate {
                node_type: "item".to_string(),
                name,
                aliases: Vec::new(),
                summary: String::new(),
                occurrence_count: count,
                first_chunk_id: first,
                latest_chunk_id: latest,
                confidence: (50 + count.saturating_mul(10)).min(90),
            }))
            .collect()
    }
}

fn strip_quantity_prefix(value: &str) -> String {
    let prefixes = ["那枚", "这枚", "一枚", "那把", "这把", "一把", "那封", "这封", "一封"];
    for prefix in prefixes {
        if let Some(stripped) = value.strip_prefix(prefix) {
            return stripped.to_string();
        }
    }
    value.to_string()
}
```

- [ ] **Step 4: 创建 `crates/core/src/analysis/foreshadow.rs`**

```rust
use std::collections::BTreeSet;
use super::extractor::{ChunkInfo, Extraction, Extractor, ForeshadowCandidate};

#[derive(Default)]
pub struct ForeshadowExtractor;

impl Extractor for ForeshadowExtractor {
    fn name(&self) -> &'static str {
        "foreshadow"
    }

    fn extract(&self, chunks: &[ChunkInfo]) -> Vec<Extraction> {
        let markers = [
            "秘密", "线索", "预感", "总觉得", "似乎", "好像",
            "日后", "终有一日", "钥匙", "信物", "谜",
        ];
        let mut items = Vec::new();
        let mut seen = BTreeSet::new();

        for chunk in chunks {
            for sentence in split_sentences(&chunk.content) {
                if !markers.iter().any(|m| sentence.contains(m)) {
                    continue;
                }
                let title = sentence.chars().take(28).collect::<String>();
                let key = format!("{}:{}", chunk.chunk_id, title);
                if !seen.insert(key) {
                    continue;
                }
                items.push(Extraction::Foreshadow(ForeshadowCandidate {
                    title,
                    foreshadow_type: "explicit_clue".to_string(),
                    first_chunk_id: chunk.chunk_id.clone(),
                    latest_chunk_id: chunk.chunk_id.clone(),
                    risk_level: "medium".to_string(),
                    evidence: sentence.chars().take(120).collect(),
                    related_nodes: Vec::new(),
                }));
            }
        }

        items
    }
}

fn split_sentences(content: &str) -> Vec<String> {
    content.split(['。', '！', '？', '\n'])
        .map(str::trim)
        .filter(|s| s.chars().count() >= 8)
        .map(ToString::to_string)
        .collect()
}
```

- [ ] **Step 5: 创建 `crates/core/src/analysis/conflicts.rs`**

```rust
use std::collections::{BTreeMap, HashMap};
use regex::Regex;
use serde_json::json;
use super::extractor::{ChunkInfo, Extraction, Extractor, IssueCandidate};
use crate::CandidateKind;
use crate::normalize_candidate;

#[derive(Default)]
pub struct EyeColorConflictExtractor;

impl Extractor for EyeColorConflictExtractor {
    fn name(&self) -> &'static str {
        "eye_color_conflict"
    }

    fn extract(&self, chunks: &[ChunkInfo]) -> Vec<Extraction> {
        let Ok(pattern) = Regex::new(
            r"([\p{Han}]{2,4}).{0,12}(黑色|灰蓝色|蓝色|褐色|金色|红色|琥珀色).{0,8}(?:眼睛|眼眸|眸子)"
        ) else {
            return Vec::new();
        };

        let mut facts = HashMap::<String, BTreeMap<String, Vec<&ChunkInfo>>>::new();

        for chunk in chunks {
            for captures in pattern.captures_iter(&chunk.content) {
                let Some(person) = captures.get(1).map(|m| normalize_name(m.as_str())) else {
                    continue;
                };
                let Some(color) = captures.get(2).map(|m| m.as_str().to_string()) else {
                    continue;
                };
                facts.entry(person).or_default().entry(color).or_default().push(chunk);
            }
        }

        let mut results = Vec::new();
        for (person, colors) in facts {
            if colors.len() < 2 {
                continue;
            }
            let evidence: Vec<_> = colors.iter().filter_map(|(color, chunks)| {
                chunks.first().map(|chunk| {
                    json!({
                        "color": color,
                        "chunk_id": chunk.chunk_id,
                        "title": chunk.title,
                        "document_path": chunk.document_path,
                        "snippet": chunk.content.chars().take(100).collect::<String>(),
                    })
                })
            }).collect();

            if let Ok(evidence_json) = serde_json::to_string(&evidence) {
                results.push(Extraction::Issue(IssueCandidate {
                    issue_type: "character_attribute_conflict".to_string(),
                    severity: "high".to_string(),
                    title: format!("{person} 的眼睛颜色可能前后不一致"),
                    description: format!("{person} 出现了多个眼睛颜色候选，请依据正文确认。"),
                    evidence_json,
                    suggested_actions_json: serde_json::to_string(&json!([
                        "保持旧设定", "接受新设定", "标记为伪装", "标记为角色认知", "标记误报"
                    ])).unwrap_or_default(),
                }));
            }
        }

        results
    }
}

#[derive(Default)]
pub struct RepeatExpressionExtractor;

impl Extractor for RepeatExpressionExtractor {
    fn name(&self) -> &'static str {
        "repeat_expression"
    }

    fn extract(&self, chunks: &[ChunkInfo]) -> Vec<Extraction> {
        let watched_terms = ["雨夜", "沉默", "钟声", "秘密", "黑暗"];
        let mut results = Vec::new();

        for term in watched_terms {
            let hits: Vec<_> = chunks.iter().filter(|chunk| chunk.content.contains(term)).collect();
            if hits.len() < 3 {
                continue;
            }
            let evidence: Vec<_> = hits.iter().take(5).map(|chunk| {
                json!({
                    "chunk_id": chunk.chunk_id,
                    "title": chunk.title,
                    "document_path": chunk.document_path,
                    "snippet": make_snippet(&chunk.content, term),
                })
            }).collect();

            if let Ok(evidence_json) = serde_json::to_string(&evidence) {
                results.push(Extraction::Issue(IssueCandidate {
                    issue_type: "repeat_expression".to_string(),
                    severity: "low".to_string(),
                    title: format!("“{term}”反复出现"),
                    description: format!("“{term}”在多个正文片段中重复出现，可在修订时确认是否有意保留。"),
                    evidence_json,
                    suggested_actions_json: serde_json::to_string(&json!([
                        "稍后处理", "标记为有意为之", "创建修订任务", "标记误报"
                    ])).unwrap_or_default(),
                }));
            }
        }

        results
    }
}

fn normalize_name(raw: &str) -> String {
    raw.trim_matches(|ch: char| {
        ch.is_whitespace() || matches!(ch, '，' | '。' | '、' | '：' | '；')
    }).to_string()
}

fn make_snippet(content: &str, query: &str) -> String {
    content.find(query).map(|byte_start| {
        let char_start = content[..byte_start].chars().count();
        let chars: Vec<char> = content.chars().collect();
        let prefix = char_start.saturating_sub(18);
        let suffix = (char_start + query.chars().count() + 18).min(chars.len());
        chars[prefix..suffix].iter().collect()
    }).unwrap_or_else(|| content.chars().take(60).collect())
}
```

- [ ] **Step 6: 更新 `lib.rs` 中 `analyze_project` 使用 Extractor 注册表**

重构 `analyze_project`：

```rust
fn analyze_project(&self, project_id: &str) -> Result<AnalysisReport> {
    let chunks = self.storage.project_chunks(project_id)?;
    let chunk_info: Vec<ChunkInfo> = chunks.iter().map(|c| ChunkInfo {
        document_id: c.document_id.clone(),
        chunk_id: c.chunk_id.clone(),
        document_path: c.document_path.clone(),
        chunk_index: c.chunk_index,
        title: c.title.clone(),
        content: c.content.clone(),
        start_offset: c.start_offset,
        end_offset: c.end_offset,
    }).collect();

    let mut extractors: Vec<Box<dyn Extractor>> = Vec::new();
    let rules = &self.extractor_rules;

    if rules.people { extractors.push(Box::new(PersonExtractor::default())); }
    if rules.places { extractors.push(Box::new(PlaceExtractor::default())); }
    if rules.items { extractors.push(Box::new(ItemExtractor::default())); }
    if rules.foreshadows { extractors.push(Box::new(ForeshadowExtractor::default())); }
    if rules.eye_color_conflicts { extractors.push(Box::new(EyeColorConflictExtractor::default())); }
    if rules.repeat_expressions { extractors.push(Box::new(RepeatExpressionExtractor::default())); }

    let mut people = Vec::new();
    let mut places = Vec::new();
    let mut items = Vec::new();
    let mut foreshadows = Vec::new();
    let mut issues = Vec::new();

    for extractor in &extractors {
        for extraction in extractor.extract(&chunk_info) {
            match extraction {
                Extraction::Candidate(c) => {
                    match c.node_type.as_str() {
                        "person" => people.push(NewNarrativeNode {
                            node_type: c.node_type,
                            name: c.name,
                            occurrence_count: c.occurrence_count,
                            first_chunk_id: c.first_chunk_id,
                            latest_chunk_id: c.latest_chunk_id,
                            confidence: c.confidence,
                        }),
                        "place" => places.push(NewNarrativeNode { /* same */ ... }),
                        "item" => items.push(NewNarrativeNode { /* same */ ... }),
                        _ => {}
                    }
                }
                Extraction::Foreshadow(f) => foreshadows.push(NewForeshadowItem {
                    title: f.title,
                    foreshadow_type: f.foreshadow_type,
                    first_chunk_id: f.first_chunk_id,
                    latest_chunk_id: f.latest_chunk_id,
                    risk_level: f.risk_level,
                    evidence: f.evidence,
                }),
                Extraction::Issue(iss) => issues.push(NewContinuityIssue {
                    issue_type: iss.issue_type,
                    severity: iss.severity,
                    title: iss.title,
                    description: iss.description,
                    evidence_json: iss.evidence_json,
                    suggested_actions_json: iss.suggested_actions_json,
                }),
            }
        }
    }

    self.storage.upsert_narrative_nodes(project_id, &people)?;
    self.storage.upsert_narrative_nodes(project_id, &places)?;
    self.storage.upsert_narrative_nodes(project_id, &items)?;
    self.storage.upsert_foreshadow_items(project_id, &foreshadows)?;
    self.storage.upsert_continuity_issues(project_id, &issues)?;

    Ok(AnalysisReport {
        person_candidates: people.len(),
        place_candidates: places.len(),
        item_candidates: items.len(),
        foreshadow_candidates: foreshadows.len(),
        issue_count: issues.len(),
    })
}
```

注意：需要更新 `use` 导入，移除不再需要的 `extract_candidates`、`extract_foreshadows`、`extract_issues` 等旧函数，添加新导入：

```rust
use crate::analysis::extractor::{ChunkInfo, Extraction, Extractor};
use crate::analysis::{
    PersonExtractor, PlaceExtractor, ItemExtractor,
    ForeshadowExtractor, EyeColorConflictExtractor, RepeatExpressionExtractor,
};
```

- [ ] **Step 7: 删除不再使用的旧辅助函数**

从 `lib.rs` 删除以下函数：
- `extract_candidates` 
- `candidate_patterns`
- `normalize_candidate`（如果仅被旧的 extract_candidates 使用）
- `trim_quantity_prefix`（如果仅被旧的 normalize_candidate 使用）
- `trim_after_context_verb` 
- `is_candidate_name`
- `min_candidate_count`
- `candidate_confidence`
- `extract_foreshadows`
- `extract_issues`
- `extract_eye_color_conflicts`
- `extract_repeat_expression_issues`
- `split_sentences`
- 及 `CandidateAccumulator` 结构体（如果仅被旧的函数使用）

保留 `make_local_snippet`、`plain_snippet`、`profile_value`（这些仍然被使用）。

移除不再需要的导入：
- `use std::collections::{BTreeMap, BTreeSet, HashMap};` → 只剩 `BTreeMap`? 检查后决定
- `use regex::Regex;` → 如果 analysis/ 模块中使用了 regex，但 lib.rs 自身不再直接使用，可以移除
- `use serde_json::json;` → 如果不再在 lib.rs 中使用

- [ ] **Step 8: 验证编译 + 测试**

```bash
cargo check -p novellossless-core 2>&1 && cargo test -p novellossless-core 2>&1
```

Expected: all tests pass. 注意测试文件中的 test 也需要调整（它依赖于 `extract_candidates` 的行为——但 test 是通过 `scan_project` 间接测试的，所以应该仍然通过）。

- [ ] **Step 9: Commit**

```bash
git add crates/core/src/analysis/ crates/core/src/lib.rs
git commit -m "refactor(analysis): port extractors to Extractor trait, remove old inline extraction functions"
```

---

### Task 5: PersonExtractor 增强—别名、称谓、对白引用

**Files:**
- Modify: `crates/core/src/analysis/person.rs`

**Interfaces:**
- Consumes: `ChunkInfo`
- Produces: `Extraction::Candidate` with `aliases` populated

- [ ] **Step 1: 在 `PersonExtractor::extract` 中添加称谓词匹配**

在现有 regex 基础上新增称谓词模式：

```rust
fn person_patterns() -> Result<Vec<Regex>, regex::Error> {
    vec![
        // 现有：XXX说/问/道
        Regex::new(r"([\p{Han}]{2,4})(?:说|问|道|喊|低声|笑道|看着|走进|转身)"),
        // 现有：向/对/跟 XXX 说/问/道
        Regex::new(r"(?:向|对|跟)([\p{Han}]{2,4})(?:说|问|道)"),
        // 新增：称谓词 "林兄"、"沈姑娘"、"师父"、"陛下"（单人或双字）
        Regex::new(r"([\p{Han}]{1,2}(?:兄|姐|弟|妹|叔|伯|婶|嫂|娘|爷|公|子|生|师|徒|君))"),
        // 新增：对白内称呼 "\"林澈，你等等""
        Regex::new(r#""([\p{Han}]{2,4})[，,]"#),
    ].into_iter().collect()
}
```

- [ ] **Step 2: 别名聚类逻辑**

添加别名映射表。如果一个人物的别名出现在文本中，将别名关联到该人物。使用简单规则：从已知人物列表中，将别名匹配到名义相同的人物。

在 `PersonExtractor` 结构体中添加别名合并状态：

```rust
#[derive(Default)]
pub struct PersonExtractor;

impl PersonExtractor {
    fn merge_aliases(&self, seen: &mut BTreeMap<String, CandidateAccumulator>, chunks: &[ChunkInfo]) {
        // 称谓到全名的简单映射
        // 从已识别的人物中，如果某称谓包含在名字内，合并
        let known_names: Vec<String> = seen.keys().cloned().collect();
        let alias_pairs: Vec<(String, String)> = known_names.iter().flat_map(|name| {
            let mut pairs = Vec::new();
            // 如果 name 是 2 字，尝试匹配单字称谓
            if name.chars().count() == 2 {
                let first_char: String = name.chars().take(1).collect();
                pairs.push((first_char + "兄", name.clone()));
                pairs.push((first_char + "公子", name.clone()));
            }
            pairs
        }).collect();

        // 扫描文档收集别名出现
        for chunk in chunks {
            for (alias, full_name) in &alias_pairs {
                if chunk.content.contains(alias) {
                    if let Some(entry) = seen.get_mut(full_name) {
                        entry.aliases.push(alias.clone());
                        entry.count += 1;
                    }
                }
            }
        }
    }
}
```

- [ ] **Step 3: 在 extract 流程中调用 merge_aliases**

在 extract 方法的 return 语句前面添加：

```rust
self.merge_aliases(&mut seen, chunks);
```

（注意：需要将 `seen` 改为 `mut`）

- [ ] **Step 4: 填充 aliases 字段到 NarrativeNodeCandidate**

在 map 阶段：

```rust
let mut aliases: Vec<String> = acc.aliases.clone();
aliases.sort();
aliases.dedup();

Extraction::Candidate(NarrativeNodeCandidate {
    node_type: "person".to_string(),
    name,
    aliases,
    // ...
})
```

- [ ] **Step 5: 更新 `NewNarrativeNode` 转换逻辑以包含 aliases**

当前 `analyze_project` 将 `NarrativeNodeCandidate` 转换为 `NewNarrativeNode`，但 `NewNarrativeNode` 没有 aliases 字段。需要在 `NewNarrativeNode` 中添加 `aliases_json` 字段，并修改 `upsert_narrative_nodes` 以存储它。

在 `crates/storage/src/lib.rs` 的 `NewNarrativeNode` 中添加：

```rust
pub struct NewNarrativeNode {
    pub node_type: String,
    pub name: String,
    pub aliases_json: String,   // 新增
    pub occurrence_count: i64,
    pub first_chunk_id: String,
    pub latest_chunk_id: String,
    pub confidence: i64,
}
```

更新 `upsert_narrative_nodes` SQL 以包含 `aliases_json`:

```sql
INSERT INTO narrative_nodes (
    id, project_id, node_type, name, aliases_json, confidence, occurrence_count,
    first_chunk_id, latest_chunk_id, created_at, updated_at
)
VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?10)
ON CONFLICT(project_id, node_type, name) DO UPDATE SET
    aliases_json = excluded.aliases_json,
    confidence = excluded.confidence,
    occurrence_count = excluded.occurrence_count,
    latest_chunk_id = excluded.latest_chunk_id,
    updated_at = excluded.updated_at
```

更新 `analyze_project` 中的转换逻辑：

```rust
"person" => people.push(NewNarrativeNode {
    node_type: c.node_type,
    name: c.name,
    aliases_json: serde_json::to_string(&c.aliases).unwrap_or_default(),
    occurrence_count: c.occurrence_count,
    first_chunk_id: c.first_chunk_id,
    latest_chunk_id: c.latest_chunk_id,
    confidence: c.confidence,
}),
```

更新 `cli` 和 `desktop lib.rs` 中的 `NewNarrativeNode` 构造调用（如果有的话——检查）。

- [ ] **Step 6: 验证编译 + 测试**

```bash
cargo check 2>&1 && cargo test 2>&1
```

检查所有使用者已更新 `NewNarrativeNode` 的构造。

- [ ] **Step 7: 编写别名测试**

在 `crates/core/src/lib.rs` 的 `mod tests` 中添加：

```rust
#[test]
fn person_aliases_are_merged() {
    let temp = tempfile::tempdir().expect("tempdir");
    let novel_dir = temp.path().join("novel");
    std::fs::create_dir(&novel_dir).expect("dir");
    // 同时出现 "林澈" 和 "林兄"
    std::fs::write(
        novel_dir.join("001.txt"),
        "第一章 雨夜\n林澈说他在雨夜醒来。林兄，你怎么在这里？",
    ).expect("write");

    let core = NovelCore::from_storage(Storage::open_memory().expect("storage"));
    let project = core.import_project("test", &novel_dir).expect("import");
    core.scan_project(&project.id).expect("scan");

    let candidates = core.list_candidates(&project.id, Some("person"), 10).expect("list");
    // 应该合并为一条，aliases 中包含 "林兄"
    let linche = candidates.iter().find(|c| c.name == "林澈").expect("林澈 found");
    assert!(linche.occurrence_count >= 2);
}
```

- [ ] **Step 8: 验证测试**

```bash
cargo test -p novellossless-core person_aliases_are_merged -- --nocapture 2>&1
```

Expected: PASS

- [ ] **Step 9: Commit**

```bash
git add crates/core/src/analysis/person.rs crates/storage/src/lib.rs crates/core/src/lib.rs
git commit -m "feat(analysis): person alias detection, NewNarrativeNode aliases_json field"
```

---

### Task 6: ForeshadowExtractor 增强—章节间隔与风险计算

**Files:**
- Modify: `crates/core/src/analysis/foreshadow.rs`
- Modify: `crates/core/src/lib.rs` — update NewForeshadowItem conversion with related_nodes

- [ ] **Step 1: 在 `ForeshadowExtractor` 中添加章节间隔追踪**

```rust
impl Extractor for ForeshadowExtractor {
    fn extract(&self, chunks: &[ChunkInfo]) -> Vec<Extraction> {
        let markers = [/* 不变 */];
        let mut seen: BTreeMap<String, ForeshadowAccumulator> = BTreeMap::new();
        let mut name_set: BTreeSet<String> = BTreeSet::new();

        for chunk in chunks {
            for sentence in split_sentences(&chunk.content) {
                if !markers.iter().any(|m| sentence.contains(m)) {
                    continue;
                }
                let title = sentence.chars().take(28).collect::<String>();

                // 合并重复伏笔（同标题）
                seen.entry(title.clone()).or_insert_with(|| ForeshadowAccumulator {
                    first_chunk: chunk.clone(),
                    latest_chunk: chunk.clone(),
                    mention_count: 0,
                    related_names: Vec::new(),
                });

                if let Some(acc) = seen.get_mut(&title) {
                    acc.latest_chunk = chunk.clone();
                    acc.mention_count += 1;

                    // 检测伏笔文本中是否包含已知人物名
                    // 从 chunks 的所有内容中收集人物名（简单正则）
                    let name_pattern = Regex::new(r"([\p{Han}]{2,4})").unwrap();
                    for cap in name_pattern.captures_iter(&sentence) {
                        let n = cap.get(1).unwrap().as_str().to_string();
                        if n.chars().count() >= 2 {
                            acc.related_names.push(n);
                        }
                    }
                }
            }
        }

        seen.into_iter().map(|(title, acc)| {
            let gap = acc.latest_chunk.chunk_index - acc.first_chunk.chunk_index;
            let risk = calculate_risk(gap, acc.mention_count);
            let mut related: Vec<String> = acc.related_names;
            related.sort();
            related.dedup();

            Extraction::Foreshadow(ForeshadowCandidate {
                title,
                foreshadow_type: "explicit_clue".to_string(),
                first_chunk_id: acc.first_chunk.chunk_id.clone(),
                latest_chunk_id: acc.latest_chunk.chunk_id.clone(),
                risk_level: risk.to_string(),
                evidence: acc.first_chunk.content.chars().take(120).collect(),
                related_nodes: related,
            })
        }).collect()
    }
}

struct ForeshadowAccumulator {
    first_chunk: ChunkInfo,
    latest_chunk: ChunkInfo,
    mention_count: i64,
    related_names: Vec<String>,
}

fn calculate_risk(chapter_gap: i64, mention_count: i64) -> &'static str {
    let score = chapter_gap.saturating_abs().saturating_mul(2).saturating_mul(mention_count);
    if score >= 20 { "high" }
    else if score >= 10 { "medium" }
    else { "low" }
}
```

- [ ] **Step 2: 更新 `analyze_project` 中的 foreshadow 转换以传递 related_nodes**

```rust
Extraction::Foreshadow(f) => foreshadows.push(NewForeshadowItem {
    title: f.title,
    foreshadow_type: f.foreshadow_type,
    first_chunk_id: f.first_chunk_id,
    latest_chunk_id: f.latest_chunk_id,
    risk_level: f.risk_level,
    evidence: f.evidence,
    // related_nodes_json 字段需要存在于 NewForeshadowItem
    // 先在 storage 中添加该字段
}),
```

当前 `NewForeshadowItem` 没有 `related_nodes_json` 字段。需要在 storage 中添加：

```rust
pub struct NewForeshadowItem {
    pub title: String,
    pub foreshadow_type: String,
    pub first_chunk_id: String,
    pub latest_chunk_id: String,
    pub risk_level: String,
    pub evidence: String,
    pub related_nodes_json: String,  // 新增
}
```

更新 `upsert_foreshadow_items` 的 SQL：

```sql
INSERT INTO foreshadow_items (
    id, project_id, title, foreshadow_type, first_chunk_id, latest_chunk_id,
    risk_level, evidence, related_nodes_json, created_at, updated_at
)
VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?10)
ON CONFLICT(project_id, title, first_chunk_id) DO UPDATE SET
    latest_chunk_id = excluded.latest_chunk_id,
    risk_level = excluded.risk_level,
    evidence = excluded.evidence,
    related_nodes_json = excluded.related_nodes_json,
    updated_at = excluded.updated_at
```

更新所有 `NewForeshadowItem` 的构造调用（cli, desktop lib.rs 中可能需要）。

- [ ] **Step 3: 验证编译 + 测试**

```bash
cargo check 2>&1 && cargo test 2>&1
```

- [ ] **Step 4: Commit**

```bash
git add crates/core/src/analysis/foreshadow.rs crates/storage/src/lib.rs crates/core/src/lib.rs
git commit -m "feat(analysis): foreshadow chapter-gap risk calc and related_nodes tracking"
```

---

### Task 7: React Router + App.tsx 拆分

**Files:**
- Modify: `apps/desktop/package.json` — 添加 react-router-dom
- Modify: `apps/desktop/src/main.tsx` — 添加 BrowserRouter
- Create: `apps/desktop/src/routes/Dashboard.tsx` — 从 App.tsx 迁移 Dashboard 部分
- Create: `apps/desktop/src/routes/SearchView.tsx`
- Create: `apps/desktop/src/routes/ContextPack.tsx`
- Create: `apps/desktop/src/routes/Privacy.tsx`
- Create: `apps/desktop/src/components/InspectorPanel.tsx`
- Modify: `apps/desktop/src/App.tsx` — 保留 layout（sidebar + topbar + routes）

- [ ] **Step 1: 安装 react-router-dom**

```bash
cd apps/desktop && npm install react-router-dom && cd ../..
```

- [ ] **Step 2: 更新 `main.tsx` 添加 BrowserRouter**

```tsx
import { BrowserRouter } from "react-router-dom";
// ...
root.render(
  <BrowserRouter>
    <App />
  </BrowserRouter>,
);
```

- [ ] **Step 3: 创建 `src/components/InspectorPanel.tsx`**

从现有 App.tsx 提取来源证据面板：

```tsx
import { ListChecks } from "lucide-react";
import type { SearchHit } from "../tauri";

interface Props {
  selectedHit: SearchHit | null;
}

export function InspectorPanel({ selectedHit }: Props) {
  return (
    <section className="panel evidence-panel">
      <div className="panel-heading compact">
        <div>
          <h2>来源证据</h2>
          <p>所有提醒都必须能回到正文。</p>
        </div>
        <ListChecks size={22} />
      </div>

      <div className="evidence-source">{selectedHit?.title ?? "未选择片段"}</div>
      {selectedHit && (
        <div className="evidence-meta">
          <div>
            <span>来源文件</span>
            <strong>{selectedHit.documentPath}</strong>
          </div>
          <div>
            <span>片段位置</span>
            <strong>第 {selectedHit.chunkIndex + 1} 段 · {selectedHit.startOffset}-{selectedHit.endOffset}</strong>
          </div>
        </div>
      )}
      <blockquote>{selectedHit ? selectedHit.snippet.replace(/[[\]]/g, "") : "选择搜索结果后查看来源。"}</blockquote>
    </section>
  );
}
```

- [ ] **Step 4: 创建 `src/routes/Dashboard.tsx`**

从 App.tsx 的 `primary-column` 和 `inspector` 内容中拆分出 dashboard 页面。包括：
- 项目导入面板
- 6 个 metric card
- 搜索面板 + 结果列表
- 记忆候选 / 伏笔候选 / 基础问题 三个面板
- 上下文包面板
- 来源证据侧栏

接收 props：
- project state, dashboard data, candidates, foreshadows, issues
- handlers for import, scan, search, status update, context pack

- [ ] **Step 5: 简化 `App.tsx`**

只保留 sidebar + topbar + error/notice row + `<Routes>`：

```tsx
import { Route, Routes } from "react-router-dom";
import { Dashboard } from "./routes/Dashboard";
import { Characters } from "./routes/Characters";
// ...

export function App() {
  // 保留所有 state 和 handler（与现有一致）

  return (
    <div className="app-shell">
      <aside className="sidebar">
        {/* 侧边栏：brand, nav, project-switcher, privacy-box 不变 */}
        {/* 但导航按钮需要改为 Link 组件，active 根据 location.pathname 判断 */}
      </aside>

      <main className="workspace">
        <header className="topbar">
          {/* 不变 */}
        </header>

        <section className="notice-row">
          {/* 不变 */}
        </section>

        <Routes>
          <Route path="/" element={<Dashboard ... />} />
          <Route path="/content" element={<ContentView projectId={selectedProject.id} />} />
          <Route path="/characters" element={<Characters projectId={selectedProject.id} />} />
          <Route path="/foreshadows" element={<Foreshadows projectId={selectedProject.id} />} />
          <Route path="/issues" element={<Issues projectId={selectedProject.id} />} />
          <Route path="/search" element={<SearchView ... />} />
          <Route path="/context-pack" element={<ContextPack ... />} />
          <Route path="/privacy" element={<Privacy privacy={privacy} />} />
        </Routes>
      </main>
    </div>
  );
}
```

- [ ] **Step 6: 侧边栏导航使用 react-router**

更新 navigation 数组添加 path：

```tsx
const navigation = [
  { label: "项目首页", icon: Home, path: "/" },
  { label: "正文", icon: BookOpenText, path: "/content" },
  { label: "搜索", icon: Search, path: "/search" },
  { label: "人物", icon: UserRound, path: "/characters" },
  { label: "伏笔", icon: Network, path: "/foreshadows" },
  { label: "时间线", icon: Clock3, path: "/timeline" },
  { label: "冲突报告", icon: AlertTriangle, path: "/issues" },
  { label: "上下文包", icon: Archive, path: "/context-pack" },
  { label: "隐私中心", icon: LockKeyhole, path: "/privacy" },
];
```

使用 `Link` 和 `useLocation`：

```tsx
import { Link, useLocation } from "react-router-dom";

function SidebarNav() {
  const location = useLocation();
  return navigation.map((item) => (
    <Link
      to={item.path}
      className={clsx("nav-item", location.pathname === item.path && "nav-item-active")}
      key={item.label}
    >
      <item.icon size={17} />
      <span>{item.label}</span>
    </Link>
  ));
}
```

- [ ] **Step 7: 验证编译**

```bash
cd apps/desktop && npx tsc --noEmit 2>&1 && cd ../..
```

Expected: clean TypeScript compile.

- [ ] **Step 8: Commit**

```bash
git add apps/desktop/
git commit -m "feat(ui): react-router setup, App.tsx split into routes"
```

---

### Task 8: 人物卡片页

**Files:**
- Create: `apps/desktop/src/routes/Characters.tsx`

- [ ] **Step 1: 创建 `Characters.tsx`**

```tsx
import { useEffect, useState } from "react";
import { ChevronRight, UserRound } from "lucide-react";
import { listCandidates, NarrativeNode } from "../tauri";
import { StatusButtons } from "../components/StatusButtons";
import { InspectorPanel } from "../components/InspectorPanel";

interface Props {
  projectId: string;
}

export function Characters({ projectId }: Props) {
  const [characters, setCharacters] = useState<NarrativeNode[]>([]);
  const [selected, setSelected] = useState<NarrativeNode | null>(null);

  useEffect(() => {
    if (projectId && projectId !== "demo") {
      listCandidates(projectId, "person", 50).then(setCharacters);
    }
  }, [projectId]);

  return (
    <section className="content-grid">
      <div className="primary-column">
        <section className="panel">
          <div className="panel-heading">
            <h2>人物卡</h2>
            <p>从正文提取的人物候选，共 {characters.length} 条</p>
          </div>
          <div className="compact-list">
            {characters.length > 0 ? (
              characters.map((c) => (
                <article
                  className={clsx("compact-item", selected?.id === c.id && "compact-item-active")}
                  key={c.id}
                  onClick={() => setSelected(c)}
                >
                  <div>
                    <strong>{c.name}</strong>
                    <p>出现 {c.occurrenceCount} 次 · {c.sourcePath} · {c.sourceTitle}</p>
                  </div>
                  <div className="row-actions">
                    <StatusButtons
                      onConfirm={() => { /* 委托给 parent */ }}
                      onDismiss={() => { /* 委托给 parent */ }}
                    />
                    <ChevronRight size={17} />
                  </div>
                </article>
              ))
            ) : (
              <div className="empty-state small">扫描后会显示人物候选。</div>
            )}
          </div>
        </section>
      </div>
      <aside className="inspector">
        {selected ? (
          <section className="panel">
            <div className="panel-heading compact">
              <h2>{selected.name}</h2>
              <UserRound size={22} />
            </div>
            <div className="evidence-meta">
              <div><span>类型</span><strong>人物</strong></div>
              <div><span>出现次数</span><strong>{selected.occurrenceCount}</strong></div>
              <div><span>置信度</span><strong>{selected.confidence}%</strong></div>
              <div><span>来源文件</span><strong>{selected.sourcePath}</strong></div>
              <div><span>首次出现</span><strong>{selected.sourceTitle}</strong></div>
              <div><span>状态</span><strong>{statusLabel(selected.status)}</strong></div>
            </div>
            <blockquote>{selected.sourceSnippet}</blockquote>
          </section>
        ) : (
          <InspectorPanel selectedHit={null} />
        )}
      </aside>
    </section>
  );
}
```

需要创建 `StatusButtons` 共享组件和 `clsx` 导入。

- [ ] **Step 2: 创建 `src/components/StatusButtons.tsx`**

```tsx
interface Props {
  onConfirm: () => void;
  onDismiss: () => void;
}

export function StatusButtons({ onConfirm, onDismiss }: Props) {
  return (
    <div className="status-actions">
      <button type="button" onClick={(e) => { e.stopPropagation(); onConfirm(); }}>确认</button>
      <button type="button" onClick={(e) => { e.stopPropagation(); onDismiss(); }}>误报</button>
    </div>
  );
}
```

- [ ] **Step 3: 检查 `clsx` 是否已安装**

已在 `package.json` 中，无需额外操作。

- [ ] **Step 4: 验证编译**

```bash
cd apps/desktop && npx tsc --noEmit 2>&1 && cd ../..
```

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/src/routes/Characters.tsx apps/desktop/src/components/
git commit -m "feat(ui): character list page with detail inspector"
```

---

### Task 9: 伏笔账本页

**Files:**
- Create: `apps/desktop/src/routes/Foreshadows.tsx`
- Modify: `apps/desktop/src/App.tsx` — 添加路由

模式与 Characters 页相同：列表 + 详情侧栏。风险等级用颜色标识。

- [ ] **Step 1: 创建 `Foreshadows.tsx`**

```tsx
import { useEffect, useState } from "react";
import { Network } from "lucide-react";
import { listForeshadows, ForeshadowItem } from "../tauri";
import { StatusButtons } from "../components/StatusButtons";

const riskColors: Record<string, string> = {
  high: "risk-high",
  medium: "risk-medium",
  low: "risk-low",
};

export function Foreshadows({ projectId }: { projectId: string }) {
  const [items, setItems] = useState<ForeshadowItem[]>([]);
  const [selected, setSelected] = useState<ForeshadowItem | null>(null);

  useEffect(() => {
    if (projectId && projectId !== "demo") {
      listForeshadows(projectId, 50).then(setItems);
    }
  }, [projectId]);

  return (
    <section className="content-grid">
      <div className="primary-column">
        <section className="panel">
          <div className="panel-heading"><h2>伏笔账本</h2><p>共 {items.length} 条</p></div>
          <div className="compact-list">
            {items.map((item) => (
              <article className="compact-item" key={item.id} onClick={() => setSelected(item)}>
                <div>
                  <strong>{item.title}</strong>
                  <p>
                    <span className={riskColors[item.riskLevel] ?? ""}>
                      {riskLabel(item.riskLevel)}
                    </span>
                    {" · "}{item.sourcePath} · {item.evidence.slice(0, 60)}
                  </p>
                </div>
                <StatusButtons onConfirm={() => {}} onDismiss={() => {}} />
              </article>
            ))}
          </div>
        </section>
      </div>
      <aside className="inspector">
        {selected && (
          <section className="panel">
            <div className="panel-heading compact"><h2>详情</h2><Network size={22} /></div>
            <div className="evidence-meta">
              <div><span>类型</span><strong>{selected.foreshadowType}</strong></div>
              <div><span>风险</span><strong>{riskLabel(selected.riskLevel)}</strong></div>
              <div><span>状态</span><strong>{selected.status}</strong></div>
              <div><span>来源</span><strong>{selected.sourcePath}</strong></div>
              <div><span>章节</span><strong>{selected.sourceTitle}</strong></div>
            </div>
            <blockquote>{selected.evidence}</blockquote>
          </section>
        )}
      </aside>
    </section>
  );
}

function riskLabel(risk: string) {
  if (risk === "high") return "高风险";
  if (risk === "medium") return "中风险";
  if (risk === "low") return "低风险";
  return "待确认";
}
```

- [ ] **Step 2: 在 App.tsx 中添加路由**（在 Task 7 的基础上）

```tsx
<Route path="/foreshadows" element={<Foreshadows projectId={selectedProject.id} />} />
```

- [ ] **Step 3: 验证编译**

```bash
cd apps/desktop && npx tsc --noEmit 2>&1 && cd ../..
```

- [ ] **Step 4: Commit**

```bash
git add apps/desktop/src/routes/Foreshadows.tsx
git commit -m "feat(ui): foreshadow ledger page with risk color coding"
```

---

### Task 10: 冲突报告页

**Files:**
- Create: `apps/desktop/src/routes/Issues.tsx`
- Modify: `apps/desktop/src/App.tsx` — 添加路由

- [ ] **Step 1: 创建 `Issues.tsx`**

```tsx
import { useEffect, useState } from "react";
import { AlertTriangle } from "lucide-react";
import { listIssues, ContinuityIssue } from "../tauri";
import { StatusButtons } from "../components/StatusButtons";

const severityColors: Record<string, string> = {
  serious: "severity-serious",
  high: "severity-high",
  medium: "severity-medium",
  low: "severity-low",
};

export function Issues({ projectId }: { projectId: string }) {
  const [issues, setIssues] = useState<ContinuityIssue[]>([]);
  const [selected, setSelected] = useState<ContinuityIssue | null>(null);

  useEffect(() => {
    if (projectId && projectId !== "demo") {
      listIssues(projectId, 50).then(setIssues);
    }
  }, [projectId]);

  return (
    <section className="content-grid">
      <div className="primary-column">
        <section className="panel">
          <div className="panel-heading"><h2>基础问题</h2><p>共 {issues.length} 条</p></div>
          <div className="compact-list">
            {issues.map((issue) => (
              <article className="compact-item issue" key={issue.id} onClick={() => setSelected(issue)}>
                <div>
                  <strong>
                    {issue.title}
                    <span className={severityColors[issue.severity]}>
                      {severityLabel(issue.severity)}
                    </span>
                  </strong>
                  <p>{issue.description}</p>
                </div>
                <StatusButtons onConfirm={() => {}} onDismiss={() => {}} />
              </article>
            ))}
          </div>
        </section>
      </div>
      <aside className="inspector">
        {selected && (
          <section className="panel">
            <div className="panel-heading compact"><h2>详情</h2><AlertTriangle size={22} /></div>
            <div className="evidence-meta">
              <div><span>类型</span><strong>{selected.issueType}</strong></div>
              <div><span>严重度</span><strong>{severityLabel(selected.severity)}</strong></div>
              <div><span>状态</span><strong>{selected.status}</strong></div>
            </div>
            <blockquote>{selected.description}</blockquote>
          </section>
        )}
      </aside>
    </section>
  );
}

function severityLabel(s: string) {
  if (s === "serious") return "严重";
  if (s === "high") return "高";
  if (s === "medium") return "中";
  if (s === "low") return "低";
  return "信息";
}
```

- [ ] **Step 2: 添加路由和验证**

```bash
cd apps/desktop && npx tsc --noEmit 2>&1 && cd ../..
```

- [ ] **Step 3: Commit**

```bash
git add apps/desktop/src/routes/Issues.tsx
git commit -m "feat(ui): conflict/issue report page with severity display"
```

---

### Task 11: 正文浏览页 + get_document_chunks API

**Files:**
- Modify: `apps/desktop/src-tauri/src/lib.rs` — 新增 `get_document_chunks` 命令
- Modify: `apps/desktop/src/tauri.ts` — 新增 `getDocumentChunks` API
- Create: `apps/desktop/src/routes/ContentView.tsx` — 正文浏览页
- Modify: `apps/desktop/src/App.tsx` — 添加路由

- [ ] **Step 1: Tauri 命令 — `get_document_chunks`**

在 `lib.rs` 中添加：

```rust
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DocumentTreeDto {
    documents: Vec<DocumentDto>,
    chunks: Vec<ChunkDto>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DocumentDto {
    id: String,
    title: String,
    chapter_count: u32,
    word_count: u32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ChunkDto {
    id: String,
    document_id: String,
    chunk_index: u32,
    title: String,
    content: String,
    start_offset: u32,
    word_count: u32,
}

#[tauri::command]
fn get_document_chunks(
    app: tauri::AppHandle,
    project_id: String,
    document_id: Option<String>,
) -> Result<DocumentTreeDto, String> {
    let core = open_core(&app)?;
    storage 需要新增方法来获取文档列表和对应的 chunks。
}
```

在 `storage/src/lib.rs` 添加：

```rust
pub fn project_documents(&self, project_id: &str) -> Result<Vec<ProjectDocument>> {
    // 查询 documents 表
}

pub fn document_chunks(&self, document_id: &str) -> Result<Vec<ProjectChunk>> {
    // 根据 document_id 查询 chunks
}
```

定义 `ProjectDocument`：

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectDocument {
    pub id: String,
    pub path: String,
    pub title: String,
    pub chapter_count: i64,
    pub word_count: i64,
}
```

在 core 层面添加：

```rust
pub fn document_tree(&self, project_id: &str, document_id: Option<&str>) -> Result<DocumentTree> {
    // ...
}
```

定义 `DocumentTree`：

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentTree {
    pub documents: Vec<DocumentInfo>,
    pub chunks: Vec<ProjectChunk>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentInfo {
    pub id: String,
    pub path: String,
    pub title: String,
    pub chapter_count: i64,
    pub word_count: i64,
}
```

- [ ] **Step 2: 前端 API**

在 `tauri.ts` 添加：

```ts
export interface DocumentInfo {
  id: string;
  path: string;
  title: string;
  chapterCount: number;
  wordCount: number;
}

export interface ChunkInfo {
  id: string;
  documentId: string;
  chunkIndex: number;
  title: string;
  content: string;
  startOffset: number;
  wordCount: number;
}

export interface DocumentTree {
  documents: DocumentInfo[];
  chunks: ChunkInfo[];
}

export function getDocumentChunks(projectId: string, documentId?: string) {
  ensureDesktopRuntime();
  return invoke<DocumentTree>("get_document_chunks", { projectId, documentId });
}
```

- [ ] **Step 3: 创建 `ContentView.tsx`**

简单的文档 + 章节树 + 内容阅读器：

```tsx
import { useEffect, useState } from "react";
import { BookOpenText, ChevronRight } from "lucide-react";
import { getDocumentChunks, DocumentInfo, ChunkInfo } from "../tauri";

interface Props { projectId: string }

export function ContentView({ projectId }: Props) {
  const [documents, setDocuments] = useState<DocumentInfo[]>([]);
  const [selectedDoc, setSelectedDoc] = useState<string | null>(null);
  const [chunks, setChunks] = useState<ChunkInfo[]>([]);
  const [selectedChunk, setSelectedChunk] = useState<ChunkInfo | null>(null);

  useEffect(() => {
    if (projectId && projectId !== "demo") {
      getDocumentChunks(projectId).then((tree) => {
        setDocuments(tree.documents);
        setChunks(tree.chunks);
      });
    }
  }, [projectId]);

  const filteredChunks = selectedDoc
    ? chunks.filter((c) => c.documentId === selectedDoc)
    : chunks;

  return (
    <section className="content-grid">
      <div className="primary-column">
        <div className="panel">
          <div className="panel-heading"><h2>正文</h2></div>
          <div className="doc-list">
            {documents.map((doc) => (
              <button
                key={doc.id}
                className={`doc-item ${selectedDoc === doc.id ? "doc-item-active" : ""}`}
                onClick={() => setSelectedDoc(doc.id)}
              >
                <BookOpenText size={16} />
                <span>{doc.title}</span>
                <small>{doc.chapterCount} 章</small>
              </button>
            ))}
          </div>
          <div className="chunk-list">
            {filteredChunks.map((chunk) => (
              <button
                key={chunk.id}
                className={`chunk-item ${selectedChunk?.id === chunk.id ? "chunk-item-active" : ""}`}
                onClick={() => setSelectedChunk(chunk)}
              >
                {chunk.title}
                <ChevronRight size={14} />
              </button>
            ))}
          </div>
        </div>
      </div>
      <aside className="inspector">
        {selectedChunk ? (
          <section className="panel">
            <h2>{selectedChunk.title}</h2>
            <div className="evidence-meta">
              <div><span>位置</span><strong>{selectedChunk.startOffset}</strong></div>
              <div><span>字数</span><strong>{selectedChunk.wordCount}</strong></div>
            </div>
            <div className="content-text">{selectedChunk.content}</div>
          </section>
        ) : (
          <div className="empty-state">选择章节后查看正文。</div>
        )}
      </aside>
    </section>
  );
}
```

- [ ] **Step 4: 验证编译（Rust + TypeScript）**

```bash
cargo check 2>&1 && cd apps/desktop && npx tsc --noEmit 2>&1 && cd ../..
```

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/src-tauri/src/lib.rs apps/desktop/src/tauri.ts apps/desktop/src/routes/ContentView.tsx crates/storage/src/lib.rs crates/core/src/lib.rs
git commit -m "feat(ui): content browser page with document tree API"
```

---

### Task 12: 搜索页提级

**Files:**
- Create: `apps/desktop/src/routes/SearchView.tsx`
- Modify: `apps/desktop/src/App.tsx` — 添加路由

从现有 App.tsx 提取搜索组件并提级为独立页面。

- [ ] **Step 1: 创建 `SearchView.tsx`**

```tsx
import { useState } from "react";
import { FileSearch, Search, ChevronRight } from "lucide-react";
import { searchProject, SearchHit } from "../tauri";
import { InspectorPanel } from "../components/InspectorPanel";

interface Props { projectId: string }

export function SearchView({ projectId }: Props) {
  const [query, setQuery] = useState("");
  const [hits, setHits] = useState<SearchHit[]>([]);
  const [selected, setSelected] = useState<SearchHit | null>(null);
  const [loading, setLoading] = useState(false);

  async function handleSearch() {
    if (!query.trim() || !projectId || projectId === "demo") return;
    setLoading(true);
    try {
      const results = await searchProject(projectId, query.trim(), 20);
      setHits(results);
      setSelected(results[0] ?? null);
    } finally {
      setLoading(false);
    }
  }

  return (
    <section className="content-grid">
      <div className="primary-column">
        <section className="panel search-panel">
          <div className="panel-heading compact">
            <h2>全文搜索</h2>
            <FileSearch size={22} />
          </div>
          <div className="search-row">
            <Search size={18} />
            <input value={query} onChange={(e) => setQuery(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && handleSearch()}
              placeholder="输入人物、物件、地点或句子" />
            <button onClick={handleSearch} disabled={loading}>
              {loading ? "搜索中" : "搜索"}
            </button>
          </div>
          <div className="results-list">
            {hits.length > 0 ? hits.map((hit) => (
              <button key={hit.chunkId}
                className={`result-item ${selected?.chunkId === hit.chunkId ? "result-item-active" : ""}`}
                onClick={() => setSelected(hit)}>
                <div>
                  <strong>{hit.title}</strong>
                  <span className="result-meta">{hit.documentPath} · 第 {hit.chunkIndex + 1} 段</span>
                  <p>{hit.snippet}</p>
                </div>
                <ChevronRight size={17} />
              </button>
            )) : (
              <div className="empty-state"><strong>{query ? "没有匹配片段" : "等待搜索"}</strong></div>
            )}
          </div>
        </section>
      </div>
      <aside className="inspector">
        <InspectorPanel selectedHit={selected} />
      </aside>
    </section>
  );
}
```

- [ ] **Step 2: 添加路由 + 验证编译**

```tsx
<Route path="/search" element={<SearchView projectId={selectedProject.id} />} />
```

```bash
cd apps/desktop && npx tsc --noEmit 2>&1 && cd ../..
```

- [ ] **Step 3: Commit**

```bash
git add apps/desktop/src/routes/SearchView.tsx
git commit -m "feat(ui): standalone search page with evidence inspector"
```

---

## 依赖图

```
Task 1 (Profile structs) ───────────────────────────────┐
    └── Task 2 (Profile integration) ───────────────────┼── Phase 1
        └── Task 3 (Extractor trait) ───────────────────┼── Phase 2
            └── Task 4 (Port extractors) ───────────────┤
                ├── Task 5 (Person enhancement) ────────┤
                └── Task 6 (Foreshadow enhancement) ────┘

Task 7 (React Router) ──────────────────────────────────┐
    ├── Task 8 (Characters page) ───────────────────────┼── Phase 3
    ├── Task 9 (Foreshadows page) ──────────────────────┤
    ├── Task 10 (Issues page) ──────────────────────────┤
    ├── Task 11 (Content browser + API) ────────────────┤
    └── Task 12 (Search page) ──────────────────────────┘
```

Phase 1 和 Phase 2 顺序依赖。Phase 3 与 Phase 1+2 独立，可并行。

## 验证

每 Task 后运行：
- Rust: `cargo check && cargo test`
- TypeScript: `cd apps/desktop && npx tsc --noEmit`
- 格式: `cargo fmt`
- 端到端: `cargo test -p novellossless-core` (核心测试覆盖了 import+scan+search 流程)
