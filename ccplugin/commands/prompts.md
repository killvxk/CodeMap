---
description: 将 codemap 使用规范注入到项目的 CLAUDE.md 中
---

# CodeMap Prompts — 注入使用规范到 CLAUDE.md

将 codemap 的使用规范写入当前项目的 `CLAUDE.md`，使每次会话自动遵循图谱优先的工作流。

## 执行步骤

### 1. 检测图谱是否存在

```bash
ls .codemap/graph.json 2>/dev/null && echo "CODEMAP_EXISTS" || echo "NO_CODEMAP"
```

如果不存在，提示用户先执行 `/codemap:scan` 生成图谱，然后**中止**。

### 2. 读取项目概览

使用 Read 工具读取 `.codemap/slices/_overview.json`，提取以下动态信息：

- `project.name` — 项目名称
- `summary.totalFiles` — 源文件总数
- `summary.languages` — 语言分布（如 `rust: 34, typescript: 2`）
- `summary.modules` — 模块列表

### 3. 生成规范段落

用上一步的动态信息填充以下模板：

```markdown
## CodeMap

本项目已启用 [CodeMap](https://github.com/killvxk/CodeMap) 代码图谱。

**项目概览**: {project_name} | {total_files} 个源文件 | 语言: {languages} | 模块: {modules}

### 使用规范

- 若 `.codemap/` 不存在，先执行 `/codemap:scan` 生成图谱
- 会话开始时执行 `/codemap:load` 加载项目上下文
- 代码结构搜索（函数、类、调用关系）优先使用 `/codemap:query`，仅当需要纯文本/正则搜索时才回退到 Grep/Glob
- 代码变更后执行 `/codemap:update` 增量更新图谱
- 重构前执行 `/codemap:impact <目标>` 评估影响范围
- 图谱过期时（commit hash 不匹配或长时间未更新）应先更新再查询
```

将模板中的占位符替换为实际值：
- `{project_name}` → `project.name`
- `{total_files}` → `summary.totalFiles`
- `{languages}` → 将 `summary.languages` 格式化为 `lang1(N), lang2(N)` 的形式
- `{modules}` → 将 `summary.modules` 数组用逗号连接

### 4. 写入 CLAUDE.md

检查当前项目根目录下的 `CLAUDE.md`：

#### 情况 A：文件不存在

使用 Write 工具创建 `CLAUDE.md`，内容为生成的规范段落。

#### 情况 B：文件存在但无 `## CodeMap` 段落

使用 Read 工具读取现有内容，确认不包含 `## CodeMap` 行。然后用 Edit 工具在文件末尾追加规范段落（前面加一个空行分隔）。

#### 情况 C：文件存在且有 `## CodeMap` 段落

使用 Read 工具读取现有内容，定位 `## CodeMap` 行的起始位置。找到该段落的结束位置（下一个同级或更高级标题 `## ` 的前一行，或文件末尾）。用 Edit 工具将旧段落替换为新生成的段落，实现幂等更新。

### 5. 展示结果

向用户输出：

```
✓ CodeMap 使用规范已写入 CLAUDE.md
  项目: {project_name}
  模块: {modules}
  规则: 6 条

下次会话将自动遵循图谱优先的工作流。
```
