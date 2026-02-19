---
name: codemap-update
description: >
  Use after code has been modified, after git commits, when the code graph
  might be outdated, or when the user says "更新图谱", "同步", "refresh",
  "update map", "代码改了", "图谱过期", "刷新", "重新索引变更".
  Also use when /load detects the graph commit hash doesn't match current HEAD.
---

# CodeMap Update -- 增量更新图谱

基于文件哈希比较，只重新解析变更的文件，将更新合并到现有图谱。

## 执行步骤

### 1. 执行增量更新

```bash
node "${CLAUDE_PLUGIN_ROOT}/cli/bin/codegraph.js" update
```

CLI 会自动：
- 对比现有文件哈希与磁盘上的当前哈希
- 只重新 AST 解析新增和修改的文件
- 从图谱中移除已删除的文件
- 重新计算模块依赖关系
- 重新生成受影响的切片

### 2. 展示变更摘要

向用户报告 CLI 输出中的信息：
- 新增文件数 (+N)
- 修改文件数 (~N)
- 删除文件数 (-N)
- 更新耗时

### 3. 刷新已加载的上下文

如果当前会话已经通过 `/load` 加载了某些模块的图谱，且这些模块受到更新影响，重新执行 `/load` 刷新上下文。
