---
description: 在代码图谱中查询函数、类、类型、变量的定义位置和调用关系
arguments:
  - name: symbol
    description: 要查询的符号名称（函数名、类名、类型名、变量名）
    required: true
  - name: type
    description: "过滤符号类型: function, class, type, variable"
    required: false
---

# CodeMap Query — 符号查询

在代码图谱中搜索函数、类、类型、变量或模块的定义和关联信息。

## 执行步骤

### 1. 执行查询

```bash
"${CLAUDE_PLUGIN_ROOT}/bin/codegraph" query "{{symbol}}" {{#type}}--type {{type}}{{/type}}
```

### 2. 展示结果

向用户展示：
- 符号类型（函数/类/接口/类型别名/变量）
- 定义位置（文件:行号）
- 函数签名（如果是函数）或变量声明（如果是变量）
- 调用者和被调用者（含行号级引用信息）
- 所属模块

### 3. 深入查看

如果用户需要看源码细节，根据查询结果的文件路径和行号范围，使用 Read 工具读取对应的源码段落。只读取精确需要的代码，而不是整个文件。
