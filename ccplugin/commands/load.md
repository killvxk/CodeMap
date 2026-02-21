---
name: codemap-load
description: 从 .codemap/ 加载代码图谱到当前会话上下文，支持加载概览、指定模块或文件
arguments:
  - name: target
    description: 模块名或文件路径。不指定则加载项目概览
    required: false
---

# CodeMap Load — 智能加载代码图谱

从 `.codemap/` 读取已缓存的代码图谱，按需注入上下文。相比全量读取源码，可节省约 95% token。

## 执行步骤

### 1. 检测图谱是否存在

```bash
ls .codemap/graph.json 2>/dev/null && echo "CODEMAP_EXISTS" || echo "NO_CODEMAP"
```

如果不存在，建议用户先执行 `/codemap:codemap-scan` 生成图谱。

### 2. 检查图谱新鲜度

```bash
"${CLAUDE_PLUGIN_ROOT}/bin/codegraph" status
```

如果图谱可能过期（commit hash 不匹配或距离上次更新很久），建议先执行 `/codemap:codemap-update`。

### 3. 加载策略

#### 无参数

加载项目概览（约 500 token）。使用 Read 工具读取 `.codemap/slices/_overview.json`。

#### 带模块名: `<target>` 是模块名

```bash
"${CLAUDE_PLUGIN_ROOT}/bin/codegraph" slice {{target}} --with-deps
```

加载目标模块的完整切片 + 依赖模块概览。

#### 带文件路径: `<target>` 是文件路径

查找该文件所属模块，加载该模块切片。
