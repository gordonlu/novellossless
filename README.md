# novellossless

本地优先的长篇小说记忆与创作控制助手。

## 是什么

novellossless 不是"替作者写小说"的 AI 工具，而是一个帮助作者在长篇创作中守住设定、伏笔、时间线、改稿影响和创作节奏的本地桌面应用。

核心原则：
- 不连 AI 也能用（所有核心功能离线可用）
- 不上传小说正文
- 所有提醒必须有来源证据
- 不替作者自动改文

## 当前功能状态

### 项目与扫描
- 创建/导入项目（支持单文件、多文件目录）
- TXT / Markdown 扫描（UTF-8、GBK、GB18030 自动识别）
- 章节识别（第一章、第1章、Chapter 1、楔子 等）
- 增量扫描 + 文件变更监听（800ms 去抖）
- 改稿记录与版本 Diff

### 记忆卡片
- 人物候选抽取与确认/误报标记
- 地点候选抽取
- 物件候选抽取
- 来源证据定位与跳转

### 创作审查
- 伏笔候选与伏笔账本
- 冲突报告（人物属性冲突、规则冲突、时间异常）
- 重复描写检测（5 种检测器）
- 设定规则系统（手动创建 + 自动抽取）
- 时间线抽取与异常检测
- 修订任务（搜索结果可一键建任务；扫描时自动从冲突/伏笔创建）
- 改稿影响分析

### 上下文包
- 按关键词查询生成上下文包（Markdown 导出）
- 全项目分析报告（Markdown 导出）

### 创作模式包
- 通用长篇模式（默认启用）
- 爽文模式（战力倒退、爽点密度等检查）
- 历史考据模式（时代穿帮、官职品级检查）
- 模式包可组合启用

### 桌面应用（Tauri + React）
- 隐私中心（离线模式、数据库路径、AI 设置）
- 系统托盘 + 关闭至托盘
- 备份与恢复
- 内置示例项目（三国演义 5 章）
- 设置页（界面、扫描、隐私、AI 配置、备份）

## 快速开始

```bash
# 运行全部测试
cargo test

# 运行所有 crate 编译检查
cargo check --workspace

# 启动桌面应用（需要 Node.js）
cd apps/desktop
pnpm install
pnpm dev
```

CLI 模式：

```bash
cargo run -p novellossless-cli -- --db novellossless.db init
cargo run -p novellossless-cli -- --db novellossless.db scan --project <id>
cargo run -p novellossless-cli -- --db novellossless.db tasks --project <id>
```

## 技术栈

| 层 | 技术 |
|---|---|
| 桌面壳 | Tauri 2 |
| 前端 | React + Vite |
| 后端 | Rust（多 crate 工作区） |
| 存储 | SQLite（rusqlite bundled） |
| 搜索 | SQLite FTS5 + LIKE 回退（中文适配） |

## 项目结构

```
apps/
  cli/          命令行验证入口
  desktop/      Tauri 桌面应用
crates/
  core/         核心编排逻辑（扫描、分析、搜索、AI provider 管理）
  storage/      SQLite 存储层
  parser/       文本解码与章节拆分
  profiles/     创作模式包加载与运行时
  rules/        设定规则引擎
  timeline/     时间线抽取与冲突检测
  tasks/        修订任务管理
  impact/       改稿影响分析
  repeated/     重复描写检测（5 种检测器）
  ai/           AI Provider 接口
profiles/
  common_longform/  通用长篇模式
  shuangwen/        爽文模式
  history/          历史考据模式
  demo/             内置示例项目
```


