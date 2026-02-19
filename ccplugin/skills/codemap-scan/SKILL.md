---
name: codemap-scan
description: >
  Use when the user wants to scan or index a project codebase, when .codemap/ directory
  does not exist, when starting work on a new project for the first time, or when
  the user says "扫描", "索引", "建立图谱", "scan", "index", "map codebase",
  "生成代码地图", "初始化 codemap".
  Also use when the user says they want to understand a project's full architecture
  and no .codemap/ exists yet.
---

# CodeMap Scan -- 全量代码图谱扫描

通过 AST 解析生成项目的结构化代码图谱，存储到 `.codemap/` 目录。后续会话可通过 `/load` 直接加载图谱，无需重新全量读取源码。

## 执行步骤

### 1. 检测 .codemap 是否已存在

```bash
ls .codemap/graph.json 2>/dev/null && echo "CODEMAP_EXISTS" || echo "NO_CODEMAP"
```

- 如果已存在，提醒用户：图谱已存在。建议使用 `/update` 进行增量更新。如果确认要重新全量扫描，继续执行。
- 如果不存在，继续执行扫描。

### 2. 执行全量扫描

```bash
node "${CLAUDE_PLUGIN_ROOT}/cli/bin/codegraph.js" scan .
```

扫描过程在 CLI 进程内完成，不消耗对话 token。

### 3. 读取并展示扫描摘要

使用 Read 工具读取 `.codemap/slices/_overview.json`，向用户展示：

- 项目名称
- 源文件总数与语言分布
- 检测到的模块列表（及各模块文件数、函数数）
- 入口文件
- 模块间依赖关系概览

### 4. 提示后续操作

告诉用户：
- `/load` -- 加载项目概览到上下文
- `/load <模块名>` -- 加载特定模块的详细图谱
- `/query <符号名>` -- 查询特定函数/类的定义和调用关系
- 图谱已持久化缓存，下次会话只需 `/load` 即可恢复上下文，节省约 95% token
