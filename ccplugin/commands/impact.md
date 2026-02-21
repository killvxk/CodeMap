---
name: codemap-impact
description: 分析修改某个模块或文件的影响范围，用于重构前的风险评估
arguments:
  - name: target
    description: 要分析的模块名或文件路径
    required: true
  - name: depth
    description: 依赖追踪深度，默认 3
    required: false
---

# CodeMap Impact — 变更影响分析

分析修改某个模块或文件会影响到哪些其他部分，用于重构前的风险评估。

## 执行步骤

### 1. 执行影响分析

```bash
"${CLAUDE_PLUGIN_ROOT}/bin/codegraph" impact "{{target}}" --depth {{depth:-3}}
```

`<target>` 可以是模块名（如 `auth`）或文件路径（如 `src/auth/login.ts`）。

### 2. 展示影响范围

向用户报告：
- 目标：被分析的模块/文件
- 直接依赖方：直接导入此模块/文件的模块
- 传递依赖方：间接受影响的模块（通过依赖链传播）
- 受影响文件总数
- 建议关注点：依赖链最深的文件优先关注

### 3. 给出建议

- 影响范围较小（< 5 个文件）→ 可以直接修改
- 影响范围中等（5-20 个文件）→ 建议分步重构
- 影响范围较大（> 20 个文件）→ 建议先写迁移计划
