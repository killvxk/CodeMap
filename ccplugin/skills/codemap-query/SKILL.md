---
name: codemap-query
description: >
  Use when the user asks about a specific function, class, type, or module
  in a project that has .codemap/.
  Keywords: 查找, 查询, 哪里定义, 谁调用了, 在哪个文件, find function,
  where is, who calls, definition of, 函数签名, 调用关系, 哪个模块,
  这个函数在哪, 怎么用的, 找一下.
---

# CodeMap Query -- 符号查询

在代码图谱中搜索函数、类、类型或模块的定义和关联信息。

## 执行步骤

### 1. 执行查询

```bash
node "${CLAUDE_PLUGIN_ROOT}/cli/bin/codegraph.js" query "<symbol>"
```

可选参数：
- `--type function|class|type` 过滤符号类型

### 2. 展示结果

向用户展示查询到的信息：
- 符号类型（函数/类/接口/类型别名）
- 定义位置（文件:行号）
- 函数签名（如果是函数）
- 调用者和被调用者（如果有记录）
- 所属模块

### 3. 深入查看

如果用户需要看源码细节，根据查询结果的文件路径和行号范围，使用 Read 工具读取对应的源码段落。这样只读取精确需要的代码，而不是整个文件。
