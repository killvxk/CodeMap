# CodeMap

AST-based code graph mapping plugin for [Claude Code](https://docs.anthropic.com/en/docs/claude-code). Scan your codebase once, persist a structural graph, and load compact slices in future sessions — saving ~95% tokens compared to re-reading all source files.

## Features

- **AST Parsing** — Uses tree-sitter (WASM) for accurate structural analysis, no regex guessing
- **Multi-Language** — TypeScript, JavaScript, Python, Go, Rust, Java, C, C++
- **Smart Slicing** — Project overview (~500 tokens) + per-module slices (~2-5k tokens) instead of full source (~200k+)
- **Incremental Updates** — File hash comparison detects changes; only re-parses modified files
- **Impact Analysis** — See what's affected before you refactor
- **Auto-Triggering** — Skills activate automatically based on your conversation context

## Installation

```bash
cd cli
npm install
```

Then register the plugin in Claude Code by pointing to the `.claude-plugin/` directory.

## CLI Commands

All commands are run via `node cli/bin/codegraph.js <command>`.

| Command | Description |
|---------|-------------|
| `scan <dir>` | Full AST scan, generates `.codemap/` with graph + slices |
| `status` | Show graph metadata (files, modules, last scan time) |
| `query <symbol>` | Search for functions, classes, types by name |
| `slice [module]` | Output project overview or a specific module slice as JSON |
| `update` | Incremental update — re-parse only changed files |
| `impact <target>` | Analyze which modules are affected by changing a target |

### Examples

```bash
# Scan a project
node cli/bin/codegraph.js scan /path/to/project

# Check graph status
node cli/bin/codegraph.js status

# Query a symbol
node cli/bin/codegraph.js query "handleLogin"

# Get module slice with dependencies
node cli/bin/codegraph.js slice auth --with-deps

# Incremental update after code changes
node cli/bin/codegraph.js update

# Impact analysis before refactoring
node cli/bin/codegraph.js impact auth --depth 3
```

## Skills

When installed as a Claude Code plugin, these skills auto-trigger based on conversation context:

| Skill | Triggers On |
|-------|------------|
| `/scan` | "扫描", "索引", "scan", "index", "map codebase" |
| `/load` | "加载图谱", "项目结构", "load", "code structure" |
| `/update` | "更新图谱", "refresh", "代码改了" |
| `/query` | "查找", "谁调用了", "where is", "find function" |
| `/impact` | "影响范围", "refactor impact", "change impact" |

## Supported Languages

| Language | Extensions | Extracted Structures |
|----------|-----------|---------------------|
| TypeScript | `.ts`, `.tsx` | Functions, imports, exports, classes, interfaces, type aliases |
| JavaScript | `.js`, `.jsx`, `.mjs`, `.cjs` | Functions, imports, exports, classes |
| Python | `.py` | Functions (decorated), imports, `__all__` exports, classes |
| Go | `.go` | Functions, methods (with receiver), imports, exported names, structs, type specs |
| Rust | `.rs` | Functions, impl methods, use declarations, pub exports, structs, enums, traits |
| Java | `.java` | Methods, constructors, imports, public exports, classes, interfaces, enums |
| C | `.c`, `.h` | Functions, `#include`, non-static exports, structs, enums, typedefs |
| C++ | `.cpp`, `.cc`, `.cxx`, `.hpp`, `.hh` | Qualified functions (`Class::method`), includes, classes, structs, namespaces |

## Graph Structure

Scanning produces a `.codemap/` directory:

```
.codemap/
├── graph.json          # Full structural graph
├── meta.json           # File hashes, timestamps, commit info
└── slices/
    ├── _overview.json  # Compact project overview (~500 tokens)
    ├── auth.json       # Per-module detailed slice
    ├── api.json
    └── ...
```

## Tests

```bash
cd cli
npm test
```

## License

[MIT](LICENSE)
