---
name: codemap-impact
description: >
  Use when the user wants to know the impact of changing a file or module,
  when planning refactoring, or when assessing risk of a code change.
  Keywords: 影响范围, 影响分析, 改这个会影响, refactor impact, what depends on,
  who uses this, 风险评估, change impact, 依赖分析, 会影响哪些,
  重构风险, 改动影响.
---

# CodeMap Impact -- 变更影响分析

分析修改某个模块或文件会影响到哪些其他部分，用于重构前的风险评估。

## 执行步骤

### 1. 执行影响分析

```bash
node "${CLAUDE_PLUGIN_ROOT}/cli/bin/codegraph.js" impact "<target>" --depth 3
```

`<target>` 可以是模块名（如 `auth`）或文件路径（如 `src/auth/login.ts`）。

### 2. 展示影响范围

向用户报告：
- **目标**: 被分析的模块/文件
- **直接依赖方**: 直接导入此模块/文件的模块
- **传递依赖方**: 间接受影响的模块（通过依赖链传播）
- **受影响文件总数**: 所有可能需要检查的文件
- **建议关注点**: 依赖链最深的文件优先关注

### 3. 给出建议

- 如果影响范围较小（< 5 个文件），建议可以直接修改
- 如果影响范围中等（5-20 个文件），建议分步重构
- 如果影响范围较大（> 20 个文件），建议先写迁移计划
