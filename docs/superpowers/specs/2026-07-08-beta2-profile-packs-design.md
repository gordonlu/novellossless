# Beta 2：创作模式包系统 设计文档

## 概述

本设计实现 PRD 第 10 节（创作模式包系统）、第 11 节（爽文模式）、第 12 节（历史考据模式）以及第 17.4 节（模式包表）。

产品原则：核心能力 + 创作模式包。core 提供通用基础设施，模式包定义不同题材下的专属规则、指标和检查。

## 1. 代码架构

### 1.1 新增 crate

```
crates/profiles/           # 新建模式包运行时 crate
```

### 1.2 profile 目录结构

```
profiles/
  common_longform/         # 通用长篇（已存在，升级）
    profile.toml           # 补全 version, description, entities, facts, events
    rules.toml             # 现有，不变
  shuangwen/               # 爽文模式（新建）
    profile.toml           # 完整 manifest
    metrics.toml           # 爽点指标定义
    rules.toml             # 抽取规则
  history/                 # 历史考据模式（新建）
    profile.toml           # 完整 manifest
    rules.toml             # 考据检查规则
    knowledge/             # 知识包目录（新建）
      tang_officials.toml  # 唐朝官职表
      tang_places.toml     # 唐朝地名表
```

### 1.3 workspace 注册

```toml
# Cargo.toml (workspace)
members = [
    "apps/cli",
    "apps/desktop/src-tauri",
    "crates/core",
    "crates/parser",
    "crates/storage",
    "crates/profiles",
]

[dependencies]
novellossless-profiles = { path = "crates/profiles" }
```

### 1.4 依赖图

```
desktop → core → storage
                ↘ profiles
                  → storage (profile_metrics table)
```

`profiles` crate 不应依赖 `core`。`core` 依赖 `profiles`，在 scan 管线中调用 profiles 的计算。

## 2. 存储层

### 2.1 新增表

在 `crates/storage/src/lib.rs` 的 `init()` 中添加：

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
    profile_id TEXT NOT NULL REFERENCES profiles(id) ON DELETE CASCADE,
    metric_type TEXT NOT NULL,
    document_id TEXT REFERENCES documents(id) ON DELETE CASCADE,
    value_json TEXT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS story_templates (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    profile_id TEXT NOT NULL REFERENCES profiles(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    template_type TEXT NOT NULL,
    structure_json TEXT NOT NULL,
    source_refs_json TEXT NOT NULL DEFAULT '[]',
    notes TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS knowledge_packs (
    id TEXT PRIMARY KEY,
    profile_id TEXT NOT NULL REFERENCES profiles(id) ON DELETE CASCADE,
    pack_name TEXT NOT NULL,
    pack_type TEXT NOT NULL,
    entries_json TEXT NOT NULL,
    version TEXT NOT NULL DEFAULT '0.1.0',
    created_at TEXT NOT NULL
);
```

### 2.2 Storage 方法

```rust
// profiles
fn upsert_profile(&self, profile: &NewProfile) -> Result<()>;
fn list_available_profiles(&self) -> Result<Vec<ProfileManifest>>;
fn get_project_profiles(&self, project_id: &str) -> Result<Vec<String>>;  // enabled_profiles_json
fn set_project_profiles(&self, project_id: &str, profile_ids: &[&str]) -> Result<()>;

// profile_metrics
fn upsert_profile_metric(&self, metric: &NewProfileMetric) -> Result<()>;
fn get_profile_metrics(&self, project_id: &str, profile_id: &str) -> Result<Vec<ProfileMetric>>;

// story_templates
fn upsert_story_template(&self, template: &NewStoryTemplate) -> Result<()>;
fn list_story_templates(&self, project_id: &str, profile_id: &str) -> Result<Vec<StoryTemplate>>;

// knowledge_packs
fn upsert_knowledge_pack(&self, pack: &NewKnowledgePack) -> Result<()>;
fn get_knowledge_packs(&self, profile_id: &str, pack_type: &str) -> Result<Vec<KnowledgePack>>;
```

### 2.3 对应 DTO 数据结构

```rust
pub struct NewProfile {
    pub id: String,
    pub name: String,
    pub version: String,
    pub path: String,
    pub enabled: bool,
}

pub struct NewProfileMetric {
    pub profile_id: String,
    pub project_id: String,
    pub metric_type: String,          // e.g. "爽点密度", "冲突频次"
    pub document_id: Option<String>,
    pub value_json: String,           // flexible numeric or structured value
}

pub struct ProfileMetric {
    pub id: String,
    pub profile_id: String,
    pub metric_type: String,
    pub document_id: Option<String>,
    pub value: String,
    pub created_at: String,
}

pub struct NewStoryTemplate {
    pub profile_id: String,
    pub project_id: String,
    pub title: String,
    pub template_type: String,
    pub structure_json: String,
    pub source_refs_json: String,
    pub notes: String,
}

pub struct NewKnowledgePack {
    pub profile_id: String,
    pub pack_name: String,
    pub pack_type: String,        // "officials", "places", "era_names", etc.
    pub entries_json: String,
    pub version: String,
}
```

## 3. `crates/profiles/` 设计

### 3.1 模块结构

```
crates/profiles/src/
  lib.rs           # public API re-exports
  loader.rs        # ProfileLoader - 扫描目录、解析 manifest
  manifest.rs      # ProfileManifest 完整 schema
  rule_engine.rs   # RuleEngine - 合并多个模式规则
  metrics.rs       # MetricRegistry - 指标声明与计算
  checks.rs        # IssueEmitter - 检查发射
  knowledge.rs     # KnowledgePackLoader - 知识包加载
```

### 3.2 ProfileManifest（完整 schema）

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct ProfileManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
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

    #[serde(default)]
    pub reports: ReportDefs,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct EntityTypes {
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
// ... etc
```

### 3.3 ProfileLoader

```rust
pub struct ProfileLoader;

impl ProfileLoader {
    /// 扫描 profiles/ 下所有子目录，加载每个 profile.toml
    pub fn load_all(profiles_root: &Path) -> Result<Vec<ProfileManifest>>;

    /// 加载单个 profile 的 rules.toml
    pub fn load_rules(profiles_root: &Path, profile_id: &str) -> Result<Option<ProfileRules>>;

    /// 加载单个 profile 的 metrics.toml
    pub fn load_metrics_toml(profiles_root: &Path, profile_id: &str) -> Result<Option<MetricsToml>>;

    /// 加载单个 profile 的 knowledge packs
    pub fn load_knowledge_packs(profiles_root: &Path, profile_id: &str) -> Result<Vec<KnowledgePackEntry>>;
}
```

`load_all` 发现逻辑：读取 `profiles/` 下所有一级子目录，检查是否存在 `profile.toml`，解析为 `ProfileManifest`。

### 3.4 RuleEngine

```rust
pub struct RuleEngine {
    extractors: ExtractorRules,
    // 未来扩展：人物配置、分析规则
}

impl RuleEngine {
    /// 合并多个 profile 的 rules.toml 规则
    /// 如果有多个 profile 指定同一规则，采用 OR 合并（任一启用即启用）
    pub fn merge_rules(profiles: &[ProfileManifest], profiles_root: &Path) -> Result<Self>;

    /// 获取最终合并的抽取器规则
    pub fn extractor_rules(&self) -> &ExtractorRules;
}
```

### 3.5 MetricRegistry

```rust
pub struct MetricRegistry {
    metrics: Vec<MetricDefinition>,
}

pub struct MetricDefinition {
    pub metric_type: String,
    pub profile_id: String,
    pub name: String,
    pub description: String,
    pub compute_fn: Option<fn(&[ProjectChunk]) -> f64>,
}

impl MetricRegistry {
    /// 从已启用的 profile 中收集指标定义
    pub fn from_profiles(profiles: &[ProfileManifest], profiles_root: &Path) -> Result<Self>;

    /// 计算所有指标
    pub fn compute_all(&self, chunks: &[ProjectChunk]) -> Vec<MetricResult>;

    /// 计算指定指标的聚合值
    pub fn compute(&self, metric_type: &str, chunks: &[ProjectChunk]) -> Option<f64>;
}
```

第一轮实现的真实指标：

| 指标 | Profile | 计算方法 |
|------|---------|---------|
| 爽点密度 | shuangwen | 每千字中"打脸""震惊""碾压"等爽点词出现次数 |
| 冲突频次 | shuangwen | 每千字中冲突/对抗词汇频率 |
| 升级间隔 | shuangwen | 相邻"晋级""突破"词汇的章节间隔 |
| 时代穿帮风险 | history | 非当代词汇（如"手机"写进唐朝）的密度 |
| 官职冲突 | history | 同一人物被不同官职描述的次数 |

### 3.6 IssueEmitter

```rust
pub struct IssueEmitter;

impl IssueEmitter {
    /// 根据已启用 profile 的 check 清单，对 chunk 发出 issue
    pub fn emit(
        checks: &[CheckDefinition],
        chunks: &[ProjectChunk],
        knowledge: &KnowledgePackIndex,
    ) -> Vec<NewContinuityIssue>;
}
```

第一轮实现的检查：

| 检查 | Profile | 逻辑 |
|------|---------|------|
| 战力倒退检查 | shuangwen | 检测人物境界/战力关键词是否出现降级描述 |
| 连续低爽点章节 | shuangwen | 连续 3 章爽点密度低于阈值 |
| 时代穿帮检查 | history | 检测唐朝背景下出现跨时代词汇 |
| 官职品级冲突 | history | 同一人物在不同章节被赋予不同品级官职 |

### 3.7 KnowledgePack

```rust
pub struct KnowledgePackLoader;

pub struct KnowledgePackEntry {
    pub pack_type: String,
    pub entries: Vec<KnowledgeItem>,
}

pub struct KnowledgeItem {
    pub term: String,
    pub category: String,
    pub metadata: HashMap<String, String>,
    // 例如：{ "dynasty": "唐", "period": "天宝三载", "rank": "正三品" }
}

impl KnowledgePackLoader {
    /// 从 profiles/<id>/knowledge/ 加载所有 TOML 知识包
    pub fn load_all(profiles_root: &Path, profile_id: &str) -> Result<Vec<KnowledgePackEntry>>;

    /// 构建跨 profile 的知识索引（用于时代穿帮检查）
    pub fn build_index(packs: &[KnowledgePackEntry]) -> KnowledgePackIndex;
}
```

#### 唐朝知识包示例 (`profiles/history/knowledge/tang_officials.toml`)

```toml
[[entry]]
term = "尚书"
category = "官职"
dynasty = "唐"
rank = "正三品"
note = "尚书省长官，唐后期多为虚衔"

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
```

```toml
# tang_places.toml
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
note = "贞观十四年置，统龟兹、疏勒等"
```

## 4. Core 集成

### 4.1 NovelCore 变更

```rust
use novellossless_profiles::{ProfileLoader, RuleEngine, MetricRegistry, IssueEmitter, KnowledgePackLoader};

pub struct NovelCore {
    storage: Storage,
    profiles: Vec<ProfileConfig>,           // ← 保留，用作运行时 ProfileConfig
    profile_manifests: Vec<ProfileManifest>, // ← 新增，完整 manifest
    extractor_rules: ExtractorRules,
    people_config: PeopleConfig,
}
```

#### `NovelCore::open()` 变更

```rust
pub fn open(db_path: &Path) -> Result<Self> {
    let storage = Storage::open(db_path)?;
    let profiles_root = find_profiles_root();
    let profiles = load_profiles_from(&profiles_root);
    let manifests = ProfileLoader::load_all(&profiles_root)?;
    let analysis_rules = profile::load_analysis_rules(&profiles_root);
    // ...
}
```

#### 新增方法

```rust
impl NovelCore {
    // 获取所有可用模式包（含完整 manifest 数据）
    pub fn get_available_profiles(&self) -> Result<Vec<ProfileManifest>>;

    // 获取项目已启用的 profile ID 列表
    pub fn get_enabled_profiles(&self, project_id: &str) -> Result<Vec<String>>;

    // 设置项目启用 profile
    pub fn set_enabled_profiles(&self, project_id: &str, profile_ids: &[&str]) -> Result<()>;

    // 获取项目在指定 profile 下的指标
    pub fn get_profile_metrics(&self, project_id: &str, profile_id: &str) -> Result<Vec<ProfileMetric>>;

    // 计算并保存 profile 指标
    pub fn compute_profile_metrics(&self, project_id: &str) -> Result<()>;

    // 获取可用 profile 的知识包
    pub fn get_knowledge_packs(&self, profile_id: &str) -> Result<Vec<KnowledgePack>>;

    // 生成 profile 感知的 issue
    pub fn emit_profile_checks(&self, project_id: &str) -> Result<Vec<ContinuityIssue>>;
}
```

### 4.2 Scan 管线集成

在 `analyze_project()` 末尾，如果项目已启用 shuangwen 或 history 模式，追加：

```rust
// 在现有分析完成后
let enabled = self.storage.get_project_profiles(project_id)?;
if enabled.iter().any(|p| p == "shuangwen" || p == "history") {
    let manifest: Vec<ProfileManifest> = self.profile_manifests
        .iter()
        .filter(|m| enabled.contains(&m.id))
        .cloned()
        .collect();

    // 计算 profile 指标
    if let Ok(registry) = MetricRegistry::from_profiles(&manifest, &profiles_root) {
        let results = registry.compute_all(&chunk_info);
        for r in results {
            self.storage.upsert_profile_metric(...)?;
        }
    }

    // 发射 profile 检查
    let knowledge = if enabled.contains(&"history".to_string()) {
        let packs = KnowledgePackLoader::load_all(&profiles_root, "history")?;
        KnowledgePackLoader::build_index(&packs)
    } else {
        KnowledgePackIndex::default()
    };
    let check_issues = IssueEmitter::emit(&manifest, &chunk_info, &knowledge);
    self.storage.upsert_continuity_issues(project_id, &check_issues)?;
}
```

## 5. Tauri 命令

### 5.1 新增命令

```rust
#[tauri::command]
fn get_available_profiles(app: tauri::AppHandle) -> Result<Vec<ProfileManifestDto>, String>;

#[tauri::command]
fn get_enabled_profiles(app: tauri::AppHandle, project_id: String) -> Result<Vec<String>, String>;

#[tauri::command]
fn set_enabled_profiles(app: tauri::AppHandle, project_id: String, profile_ids: Vec<String>) -> Result<(), String>;

#[tauri::command]
fn get_profile_metrics(app: tauri::AppHandle, project_id: String, profile_id: String) -> Result<Vec<ProfileMetricDto>, String>;

#[tauri::command]
fn get_knowledge_packs(app: tauri::AppHandle, profile_id: String) -> Result<Vec<KnowledgePackDto>, String>;
```

DTO 定义：

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
    metadata: HashMap<String, String>,
}
```

### 5.2 移除/替代

- `list_profiles` 命令 → `get_available_profiles` 替代（更丰富的数据）
- `ProfileInfo` struct → `ProfileManifest` 替代

## 6. 前端 UI

### 6.1 Settings 页新增「创作模式」区域

在 `Settings.tsx` 中添加 section：

```tsx
<section className="settings-section">
  <h3 className="settings-section-title">创作模式</h3>
  {availableProfiles.map(profile => (
    <div className="settings-row" key={profile.id}>
      <div>
        <label>{profile.name}</label>
        <p className="settings-desc">{profile.description}</p>
      </div>
      <button
        type="button"
        className={clsx("toggle", enabledProfiles.includes(profile.id) && "toggle-on")}
        onClick={() => toggleProfile(profile.id)}
      >
        <div className="toggle-knob" />
      </button>
    </div>
  ))}
</section>
```

### 6.2 新路由：模式详情页（可选）

在项目详情页面展示已启用模式的活跃指标（如爽点密度曲线）。

### 6.3 Issue 展示增强

Issue 列表标注来源 profile（哪条检查规则发射的），允许用户按模式过滤。

## 7. 操作顺序

| 层 | 组件 | 状态 |
|----|------|------|
| 配置 | common_longform/profile.toml 升级 | 改 |
| 配置 | shuangwen/ 新建 | 新 |
| 配置 | history/ 新建 | 新 |
| 配置 | history/knowledge/ 唐朝知识包 | 新 |
| 存储 | profile_metrics 表 | 新 |
| 存储 | knowledge_packs 表 | 新 |
| 存储 | story_templates 表 | 新 |
| 存储 | profiles 表 | 新 |
| 存储 | enabled_profiles_json 读写 | 补 |
| profiles crate | ProfileLoader | 新 |
| profiles crate | ProfileManifest schema | 新 |
| profiles crate | RuleEngine | 新 |
| profiles crate | MetricRegistry | 新 |
| profiles crate | IssueEmitter | 新 |
| profiles crate | KnowledgePackLoader | 新 |
| core | 集成 profiles crate | 改 |
| tauri | 5 个新命令 | 新 |
| frontend | 设置页模式选择 | 新 |

## 8. 不做（YAGNI）

- 不实现 RuleEngine 的动态规则 DSL（只用 toml 配置驱动）
- 不做报告渲染器、上下文包构建器（PRD 16.3 中的 ReportRenderer / ContextPackBuilder 留后续）
- 不做爽文套路模板存储（story_templates 表创建但不实现 UI 消费）
- 不做 CLI profiles 子命令
- 不做 AI 驱动的分析（PRD 6）
- 不做 per-document 级别的 profile 设置（按项目粒度即可）

## 9. 验证

- `cargo test`（含 storage + core + profiles crate 测试）
- 手动验证：导入三国演义 demo，启用 shuangwen 模式，观察爽点密度指标
- 手动验证：启用 history 模式，观察时代穿帮检查是否标记"
