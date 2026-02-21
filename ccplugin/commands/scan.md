---
name: codemap-scan
description: 全量扫描项目代码，生成 AST 结构化图谱到 .codemap/ 目录
arguments:
  - name: dir
    description: 要扫描的目录路径，默认为当前目录
    required: false
---

# CodeMap Scan — 全量代码图谱扫描

通过 AST 解析生成项目的结构化代码图谱，存储到 `.codemap/` 目录。

## 执行步骤

### 1. 检测图谱是否已存在

```bash
ls .codemap/graph.json 2>/dev/null && echo "CODEMAP_EXISTS" || echo "NO_CODEMAP"
```

- 如果已存在，提醒用户图谱已存在，建议使用 `/codemap:codemap-update` 增量更新。如果用户确认要重新全量扫描，继续执行。
- 如果不存在，继续执行。

### 2. 执行全量扫描

```bash
"${CLAUDE_PLUGIN_ROOT}/bin/codegraph" scan {{dir:-.}}
```

### 3. 展示扫描摘要

使用 Read 工具读取 `.codemap/slices/_overview.json`，向用户展示：
- 项目名称、源文件总数与语言分布
- 检测到的模块列表（各模块文件数、函数数）
- 入口文件、模块间依赖关系概览

### 4. 提示后续操作

- `/codemap:codemap-load` — 加载项目概览到上下文
- `/codemap:codemap-load <模块名>` — 加载特定模块详细图谱
- `/codemap:codemap-query <符号名>` — 查询函数/类的定义和调用关系
- 图谱已持久化，下次会话只需 `/codemap:codemap-load` 即可恢复上下文
