---
name: codemap-load
description: >
  Use when starting work on a project that has a .codemap/ directory, when the user
  asks about project structure or architecture, when the user wants to understand
  code before making changes, or when beginning any coding task.
  Keywords: 加载图谱, 项目结构, 架构, load, 了解代码, 开始工作, 代码地图,
  查看模块, understand codebase, project overview, code structure, 恢复上下文,
  看一下项目, 代码结构, 模块列表.
  Also use proactively at session start if .codemap/ exists in the working directory.
---

# CodeMap Load -- 智能加载代码图谱

从 `.codemap/` 读取已缓存的代码图谱，按需注入上下文。相比全量读取源码，可节省约 95% token。

## 执行步骤

### 1. 检测图谱是否存在

```bash
ls .codemap/graph.json 2>/dev/null && echo "CODEMAP_EXISTS" || echo "NO_CODEMAP"
```

如果不存在，建议用户先执行 `/scan` 生成图谱。

### 2. 检查图谱新鲜度

```bash
node "${CLAUDE_PLUGIN_ROOT}/cli/bin/codegraph.js" status
```

观察输出中的 commit hash 和 scanned at 时间。如果图谱可能过期（比如距离上次更新很久），建议先执行 `/update`。

### 3. 加载策略

#### 无参数: `/load`

加载项目概览（约 500 token）。

使用 Read 工具读取 `.codemap/slices/_overview.json`，将内容作为项目上下文提供。

#### 带模块名: `/load <module>`

加载目标模块的完整切片 + 依赖模块概览（约 2-5k token）。

```bash
node "${CLAUDE_PLUGIN_ROOT}/cli/bin/codegraph.js" slice <module> --with-deps
```

读取输出并注入上下文。

#### 带文件路径: `/load <path>`

查找该文件所属模块，加载该模块切片。

### 4. 智能推断（自动触发时）

如果用户描述了一个任务（如"修改登录功能"、"重构 API 错误处理"）：

1. 读取 `.codemap/slices/_overview.json` 获取模块列表和导出符号
2. 从用户描述中提取关键词
3. 匹配模块名或导出符号名
4. 自动加载匹配到的模块切片

示例匹配：
- "登录" / "login" / "auth" → 加载 `auth` 模块
- "API" / "路由" / "routes" → 加载 `api` 模块
- "数据库" / "db" / "database" → 加载 `db` 模块
