---
name: codemap
description: >
  CodeMap 代码图谱智能路由。自动检测项目状态并执行合适的操作。
  Use when: starting work on any project, the user mentions code structure or architecture,
  before making code changes, when the user asks about functions/classes/modules,
  when planning refactoring, or when .codemap/ directory exists or needs to be created.
  Keywords: 代码图谱, 项目结构, 架构, 代码地图, 模块, 扫描, 索引, 加载, 查询,
  影响分析, 重构, scan, load, query, impact, update, codemap, code graph,
  understand codebase, project overview, code structure, 了解代码, 开始工作,
  查找函数, 哪里定义, 谁调用了, 影响范围, 依赖分析, 更新图谱, 刷新.
---

# CodeMap — 代码图谱智能路由

CodeMap 通过 AST 解析生成项目的结构化代码图谱，后续会话可直接加载图谱而无需重新读取源码，节省约 95% token。

本 skill 是统一入口，根据项目状态和用户意图自动路由到合适的操作。

## 路由判断流程

### Step 1: 检测 .codemap/ 是否存在

```bash
ls .codemap/graph.json 2>/dev/null && echo "CODEMAP_EXISTS" || echo "NO_CODEMAP"
```

- **不存在** → 告知用户项目尚未建立代码图谱，建议执行 `/codemap:codemap-scan` 进行全量扫描。结束。

### Step 2: 检查图谱新鲜度

```bash
"${CLAUDE_PLUGIN_ROOT}/bin/codegraph" status
```

- **过期**（commit hash 不匹配 HEAD 或距上次更新超过 1 天）→ 建议先执行 `/codemap:codemap-update` 增量更新。

### Step 3: 根据用户意图路由

分析用户的消息内容，匹配以下场景：

| 用户意图 | 路由目标 |
|----------|----------|
| 会话刚开始 / 想了解项目 / 无特定需求 | 执行 `/codemap:codemap-load` 加载概览 |
| 提到特定模块名 | 执行 `/codemap:codemap-load <模块名>` |
| 问某个函数/类/类型在哪、谁调用了 | 执行 `/codemap:codemap-query <符号名>` |
| 谈到重构、改动影响、风险评估 | 执行 `/codemap:codemap-impact <目标>` |
| 说代码改了、图谱过期、要刷新 | 执行 `/codemap:codemap-update` |
| 要重新全量扫描 | 执行 `/codemap:codemap-scan` |

### Step 4: 执行路由

使用 Skill 工具调用对应的 command，或直接执行对应的 CLI 命令。

## 可用命令一览

| 命令 | 用途 |
|------|------|
| `/codemap:codemap-scan` | 全量扫描项目，生成 .codemap/ 图谱 |
| `/codemap:codemap-load [target]` | 加载图谱到上下文（概览/模块/文件） |
| `/codemap:codemap-query <symbol>` | 查询符号定义和调用关系 |
| `/codemap:codemap-update` | 增量更新图谱 |
| `/codemap:codemap-impact <target>` | 分析变更影响范围 |
