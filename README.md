# CodeMap

基于 AST 的代码图谱映射插件，适用于 [Claude Code](https://docs.anthropic.com/en/docs/claude-code)。扫描代码库一次，持久化结构图谱，后续会话加载紧凑切片即可恢复上下文——相比重新读取全部源码，节省约 95% token。

AST-based code graph mapping plugin for [Claude Code](https://docs.anthropic.com/en/docs/claude-code). Scan your codebase once, persist a structural graph, and load compact slices in future sessions — saving ~95% tokens compared to re-reading all source files.

## 特性 / Features

- **AST 解析 / AST Parsing** — 使用 tree-sitter (WASM) 进行精确的结构分析，非正则猜测 / Uses tree-sitter (WASM) for accurate structural analysis, no regex guessing
- **多语言支持 / Multi-Language** — TypeScript, JavaScript, Python, Go, Rust, Java, C, C++
- **智能切片 / Smart Slicing** — 项目概览 (~500 tokens) + 按模块切片 (~2-5k tokens)，替代全量源码 (~200k+) / Project overview (~500 tokens) + per-module slices (~2-5k tokens) instead of full source (~200k+)
- **增量更新 / Incremental Updates** — 基于文件哈希比较检测变更，仅重新解析修改的文件 / File hash comparison detects changes; only re-parses modified files
- **影响分析 / Impact Analysis** — 重构前查看哪些模块会受影响 / See what's affected before you refactor
- **自动触发 / Auto-Triggering** — Skill 根据对话上下文自动激活 / Skills activate automatically based on your conversation context

## 安装 / Installation

```bash
cd cli
npm install
```

然后在 Claude Code 中注册插件，指向 `.claude-plugin/` 目录。

Then register the plugin in Claude Code by pointing to the `.claude-plugin/` directory.

## CLI 命令 / CLI Commands

所有命令通过 `node cli/bin/codegraph.js <command>` 运行。

All commands are run via `node cli/bin/codegraph.js <command>`.

| 命令 / Command | 描述 / Description |
|---------|-------------|
| `scan <dir>` | 全量 AST 扫描，生成 `.codemap/` 图谱和切片 / Full AST scan, generates `.codemap/` with graph + slices |
| `status` | 显示图谱元信息（文件数、模块、上次扫描时间）/ Show graph metadata (files, modules, last scan time) |
| `query <symbol>` | 按名称搜索函数、类、类型 / Search for functions, classes, types by name |
| `slice [module]` | 输出项目概览或指定模块切片（JSON）/ Output project overview or a specific module slice as JSON |
| `update` | 增量更新——仅重新解析变更的文件 / Incremental update — re-parse only changed files |
| `impact <target>` | 分析修改目标会影响哪些模块 / Analyze which modules are affected by changing a target |

### 示例 / Examples

```bash
# 扫描项目 / Scan a project
node cli/bin/codegraph.js scan /path/to/project

# 检查图谱状态 / Check graph status
node cli/bin/codegraph.js status

# 查询符号 / Query a symbol
node cli/bin/codegraph.js query "handleLogin"

# 获取模块切片（含依赖）/ Get module slice with dependencies
node cli/bin/codegraph.js slice auth --with-deps

# 增量更新 / Incremental update after code changes
node cli/bin/codegraph.js update

# 影响分析 / Impact analysis before refactoring
node cli/bin/codegraph.js impact auth --depth 3
```

## Skills

作为 Claude Code 插件安装后，以下 skill 会根据对话上下文自动触发：

When installed as a Claude Code plugin, these skills auto-trigger based on conversation context:

| Skill | 触发词 / Triggers On |
|-------|------------|
| `/scan` | "扫描", "索引", "scan", "index", "map codebase" |
| `/load` | "加载图谱", "项目结构", "load", "code structure" |
| `/update` | "更新图谱", "refresh", "代码改了" |
| `/query` | "查找", "谁调用了", "where is", "find function" |
| `/impact` | "影响范围", "refactor impact", "change impact" |

## 支持的语言 / Supported Languages

| 语言 / Language | 扩展名 / Extensions | 提取结构 / Extracted Structures |
|----------|-----------|---------------------|
| TypeScript | `.ts`, `.tsx` | 函数、导入、导出、类、接口、类型别名 / Functions, imports, exports, classes, interfaces, type aliases |
| JavaScript | `.js`, `.jsx`, `.mjs`, `.cjs` | 函数、导入、导出、类 / Functions, imports, exports, classes |
| Python | `.py` | 函数（含装饰器）、导入、`__all__` 导出、类 / Functions (decorated), imports, `__all__` exports, classes |
| Go | `.go` | 函数、方法（含接收者）、导入、导出名、结构体、类型声明 / Functions, methods (with receiver), imports, exported names, structs, type specs |
| Rust | `.rs` | 函数、impl 方法、use 声明、pub 导出、结构体、枚举、trait / Functions, impl methods, use declarations, pub exports, structs, enums, traits |
| Java | `.java` | 方法、构造器、导入、public 导出、类、接口、枚举 / Methods, constructors, imports, public exports, classes, interfaces, enums |
| C | `.c`, `.h` | 函数、`#include`、非 static 导出、结构体、枚举、typedef / Functions, `#include`, non-static exports, structs, enums, typedefs |
| C++ | `.cpp`, `.cc`, `.cxx`, `.hpp`, `.hh` | 限定函数名（`Class::method`）、include、类、结构体、命名空间 / Qualified functions (`Class::method`), includes, classes, structs, namespaces |

## 图谱结构 / Graph Structure

扫描后生成 `.codemap/` 目录：

Scanning produces a `.codemap/` directory:

```
.codemap/
├── graph.json          # 完整结构图谱 / Full structural graph
├── meta.json           # 文件哈希、时间戳、提交信息 / File hashes, timestamps, commit info
└── slices/
    ├── _overview.json  # 紧凑项目概览 (~500 tokens) / Compact project overview (~500 tokens)
    ├── auth.json       # 按模块的详细切片 / Per-module detailed slice
    ├── api.json
    └── ...
```

## 测试 / Tests

```bash
cd cli
npm test
```

## 许可证 / License

[MIT](LICENSE)
