# Alpha 1: 记忆卡片循环 — 纵深现有链路

> 基于 novellossless 项目 roadmap 的 Alpha 1 阶段，聚焦加强现有最薄弱的三个环节。

## 1. 分析管线加深

### 1.1 Extractor trait + 注册表

将 `analyze_project` 中顺序调用的提取逻辑重构为 Extractor trait 模式：

```rust
pub trait Extractor {
    fn name(&self) -> &'static str;
    fn extract(&self, chunks: &[ChunkInfo], rules: &ProfileRules) -> Vec<Extraction>;
}

pub enum Extraction {
    Candidate(NarrativeNodeCandidate),
    Foreshadow(ForeshadowCandidate),
    Issue(IssueCandidate),
}
```

`analyze_project` 改为遍历所有注册的 Extractor，统一 upsert。

现有提取器全部实现此 trait：
- `PersonExtractor`
- `PlaceExtractor`
- `ItemExtractor`
- `ForeshadowExtractor`
- `EyeColorConflictExtractor`
- `RepeatExpressionExtractor`

### 1.2 人物提取增强

当前仅抓 `XXX说/道/问` 前 2-4 Han 字符。新增：

- **称谓词提取**：`林兄`、`沈姑娘`、`师父`、`陛下` → 识别为已知人物的别名
- **对白内称呼**：`"林澈，你等等"` → `林澈` 作为对白内引用
- **别名聚类**：`林澈` + `林兄` + `林公子` → 合并到同一 NarrativeNode.aliases_json
- **称谓词表**：`兄`、`姑娘`、`师父`、`公子`、`小姐`、`大人`、`陛下`、`娘娘`、`将军`

### 1.3 叙事节点填充

当前 `aliases_json`、`summary`、`first_chunk_id`、`latest_chunk_id` 全为空。

改动：
- 提取时收集所有匹配到的别名
- 取最早出现的 chunk 作为 first_chunk_id
- 取最近出现的 chunk 作为 latest_chunk_id
- summary 用 `<最早片段> ... <最近片段>` 拼接

### 1.4 伏笔追踪增强

当前仅关键词匹配（`秘密`、`线索`、`预感` 等 11 个词）。

新增：
- **章节间隔追踪**：记录每个 foreshadow 的 `first_chunk_id` 和 `latest_chunk_id`，计算间隔章节数
- **风险等级计算**：`risk = 间隔章节数 × 提及次数 × 2`，`>=20 → high`，`>=10 → medium`，其余 `low`
- **related_nodes 填充**：如果伏笔文本中包含已识别的人物名，自动关联

### 1.5 新增文件

```text
crates/core/src/analysis/
  mod.rs          → 导出所有 extractor，注册表
  extractor.rs    → Extractor trait + Extraction enum
  person.rs       → PersonExtractor (增强版)
  place.rs        → PlaceExtractor (现有直接迁移)
  item.rs         → ItemExtractor (现有直接迁移)
  foreshadow.rs   → ForeshadowExtractor (增强版)
  conflicts.rs    → EyeColorConflictExtractor + RepeatExpressionExtractor
```

### 1.6 测试

- 别名聚类测试：输入含 `林兄`、`林公子` 的文本，输出与 `林澈` 合并
- 伏笔间隔测试：第 1 章和第 10 章出现 → gap=9 → risk=medium
- 空文本测试：无候选不报错
- 现有测试保持绿色

## 2. UI 导航激活 + 卡片页

### 2.1 路由结构

新增 `react-router-dom`。路由表：

| 路径 | 页面 | 数据源 | 优先级 |
|---|---|---|---|
| `/` | 项目首页（dashboard） | 现有 | P0 |
| `/content` | 正文浏览 | 新增 `get_document_chunks` API | P0 |
| `/characters` | 人物列表 | `list_candidates(person)` | P0 |
| `/foreshadows` | 伏笔账本 | `list_foreshadows` | P0 |
| `/issues` | 冲突报告 | `list_issues` | P0 |
| `/search` | 全文搜索 | `search_project`（提级） | P0 |
| `/context-pack` | 上下文包 | 现有 | P1 |
| `/privacy` | 隐私中心 | 现有 | P1 |
| `/timeline` | 时间线 | 骨架 | P2 |

### 2.2 组件拆分

将 `App.tsx`（1087 行）拆为：

```
src/
  App.tsx              → layout only (sidebar + topbar + <Routes>)
  routes/
    Dashboard.tsx      → 现有内容直接移入
    ContentView.tsx    → 正文浏览（章节树 + 内容区）
    Characters.tsx     → 人物卡片列表
    Foreshadows.tsx    → 伏笔账本
    Issues.tsx         → 冲突报告
    SearchView.tsx     → 搜索页（现有搜索组件提级）
    ContextPack.tsx    → 上下文包页
    Privacy.tsx        → 隐私中心
  components/
    Sidebar.tsx        → 现有侧边栏逻辑
    InspectorPanel.tsx → 现有详情侧栏（复用）
    CandidateCard.tsx  → 候选人卡片
    ...
```

### 2.3 正文浏览页新增 Tauri API

```rust
#[tauri::command]
fn get_document_chunks(project_id: String, document_id: Option<String>) -> Result<DocumentTreeDto, String>
```

返回结构：

```rust
struct DocumentTreeDto {
    documents: Vec<DocumentDto>,
    chunks: Vec<ChunkDto>,  // document_id 过滤后返回
}

struct DocumentDto {
    id: String,
    title: String,       // 文件名
    chapter_count: u32,
    word_count: u32,
}

struct ChunkDto {
    id: String,
    document_id: String,
    chunk_index: u32,
    title: String,       // 章节标题
    content: String,
    start_offset: u32,
    word_count: u32,
}
```

### 2.4 页面模板

每个列表页复用同一模式：

```tsx
function ListPage<T>({ fetchItems, renderItem, renderDetail }) {
  const [items, setItems] = useState<T[]>([]);
  const [selected, setSelected] = useState<T | null>(null);
  // 左：sidebar（不变）
  // 中：items.map(renderItem)，点击 → setSelected
  // 右：selected ? renderDetail(selected) : <EmptyInspector />
}
```

### 2.5 侧边栏 active 状态

当前通过硬编码 `item.active = true` 控制。改为：

```tsx
const location = useLocation();
const isActive = (path: string) => location.pathname === path;
```

## 3. Profile 运行时入门

### 3.1 配置结构

```rust
// crates/core/src/profile.rs

#[derive(Debug, Clone, Deserialize)]
pub struct ProfileConfig {
    pub id: String,
    pub name: String,
    pub rules: ProfileRules,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProfileRules {
    pub chapter_recognition: bool,   // 默认 true
    pub full_text_search: bool,      // 默认 true
    pub evidence_required: bool,     // 默认 true
    pub auto_modify_source: bool,    // 默认 false
}

#[derive(Debug, Clone, Deserialize)]
pub struct ExtractorRules {
    pub people: bool,
    pub places: bool,
    pub items: bool,
    pub foreshadows: bool,
    pub eye_color_conflicts: bool,
    pub repeat_expressions: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PeopleConfig {
    pub min_name_length: u32,
    pub max_name_length: u32,
    pub enable_alias_detection: bool,
}
```

### 3.2 新增 `rules.toml`

写入 `profiles/common_longform/rules.toml`：

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

### 3.3 NovellCore 集成

```rust
pub struct NovelCore {
    db: Database,
    profiles: Vec<ProfileConfig>,
    extractor_rules: ExtractorRules,
    people_config: PeopleConfig,
}
```

初始化时读取 `profiles/<id>/rules.toml`，缺失则全默认值。

### 3.4 规则挂钩

| 规则 | 代码影响点 |
|---|---|
| `chapter_recognition = false` | `scan_file` 不调用 `split_chapters`，整文件作为单 chunk |
| `full_text_search = false` | `upsert_document_with_chunks` 跳过 FTS5 写入 |
| `evidence_required = true` | `Extraction` 缺 source_chunk_id 时跳过 |
| `auto_modify_source = false` | 断言或编译时保证不写原文 |
| `people = false` | `PersonExtractor` 不注册 |
| `enable_alias_detection = false` | `PersonExtractor` 不跑称谓/对白提取 |

## 4. 依赖关系

```
Phase 1 (profile 先) → Phase 2 (extractor 重构) → Phase 3 (UI 路由 + 页面)
```

三阶段可独立交付，但 profile 规则应先就位，这样 extractor 注册表可以依赖它。

## 5. 非目标

- 不引入 AI 提取
- 不构建 facts 系统（memory_facts 表留 Alpha 2）
- 不构建人物认知边界
- 不构建事件系统
- 不构建暗线系统
- 不构建增量扫描/文件监听
- 不构建报告导出
