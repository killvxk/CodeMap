[中文文档](README.zh-CN.md)

[![Release](https://github.com/killvxk/CodeMap/actions/workflows/release.yml/badge.svg)](https://github.com/killvxk/CodeMap/actions/workflows/release.yml)

# CodeMap

AST-based code graph mapping plugin for [Claude Code](https://docs.anthropic.com/en/docs/claude-code). Scan your codebase once, persist a structural graph, and load compact slices in future sessions — saving ~95% tokens compared to re-reading all source files.

## Features

- **AST Parsing** — Uses tree-sitter native bindings for accurate structural analysis, no regex guessing
- **Multi-Language** — TypeScript, JavaScript, Python, Go, Rust, Java, C, C++
- **Smart Slicing** — Project overview (~500 tokens) + per-module slices (~2-5k tokens) instead of full source (~200k+)
- **Variable Tracking** — Tracks module-level const/static/let/var declarations, queryable with `--type variable`
- **Line-Level References** — Cross-file references pinpoint import line + usage lines; same-file exported symbols also track usage locations
- **Incremental Updates** — File hash comparison detects changes; only re-parses modified files
- **Impact Analysis** — See what's affected before you refactor
- **Auto-Triggering** — Skills activate automatically based on your conversation context

---

## Installation

### Prerequisites

- **Claude Code** CLI ([Install Guide](https://docs.anthropic.com/en/docs/claude-code))

### Option 1: Install as Claude Code Plugin (Recommended)

#### 1. Clone the repository

```bash
git clone https://github.com/killvxk/CodeMap.git
cd CodeMap
```

#### 2. Binary Engine

The plugin **automatically downloads** the platform-specific binary from GitHub Releases to `~/.codemap/bin/` on first command execution. No manual steps required.

You can also install manually. Binary lookup order (highest to lowest priority):

| Priority | Location | Description |
|---|---|---|
| 1 | `PATH` | Globally installed |
| 2 | `~/.codemap/bin/` | User-level dedicated directory (recommended) |
| 3 | `ccplugin/bin/` | Plugin directory (backward compatible) |
| 4 | `rust-cli/target/release/` | Local dev build |
| 5 | Auto-download | Downloads from GitHub Releases to `~/.codemap/bin/` |

```bash
# Manual install example
mkdir -p ~/.codemap/bin
# Linux x64
curl -fSL -o ~/.codemap/bin/codegraph-x86_64-linux \
  https://github.com/killvxk/CodeMap/releases/latest/download/codegraph-x86_64-linux
chmod +x ~/.codemap/bin/codegraph-x86_64-linux

# macOS Apple Silicon
curl -fSL -o ~/.codemap/bin/codegraph-aarch64-macos \
  https://github.com/killvxk/CodeMap/releases/latest/download/codegraph-aarch64-macos
chmod +x ~/.codemap/bin/codegraph-aarch64-macos
```

> Customize the directory via `CODEMAP_HOME` env var (default `~/.codemap`).

#### 3. Install as Claude Code plugin

Run the following commands inside a Claude Code session (these are slash commands, not terminal commands):

**Option A: Install from local directory**

```
/plugin marketplace add /absolute/path/to/CodeMap
/plugin install codemap@codemap-plugins
```

**Option B: Install from GitHub**

```
/plugin marketplace add killvxk/CodeMap
/plugin install codemap@codemap-plugins
```

After installation, **restart Claude Code** for the plugin to take effect.

> **How it works:** Claude Code reads `.claude-plugin/marketplace.json` at the repo root, where `"source": "./ccplugin"` points to the plugin directory. It then loads `ccplugin/.claude-plugin/plugin.json` and auto-discovers commands in `ccplugin/commands/`, skills in `ccplugin/skills/`, and hooks in `ccplugin/hooks/`.

#### 4. Verify plugin installed

After restarting Claude Code, type:

```
/codemap:scan
```

If the plugin is installed correctly, this command will trigger the code scan workflow.

#### Uninstall

```
/plugin uninstall codemap@codemap-plugins
```

### Option 2: Download Pre-compiled Binary

Download the binary for your platform from [GitHub Releases](https://github.com/killvxk/CodeMap/releases) and place it in `~/.codemap/bin/` or anywhere in your PATH:

```bash
# Linux x64
mkdir -p ~/.codemap/bin
curl -fSL -o ~/.codemap/bin/codegraph-x86_64-linux \
  https://github.com/killvxk/CodeMap/releases/latest/download/codegraph-x86_64-linux
chmod +x ~/.codemap/bin/codegraph-x86_64-linux

# macOS (Apple Silicon)
mkdir -p ~/.codemap/bin
curl -fSL -o ~/.codemap/bin/codegraph-aarch64-macos \
  https://github.com/killvxk/CodeMap/releases/latest/download/codegraph-aarch64-macos
chmod +x ~/.codemap/bin/codegraph-aarch64-macos

# Windows (PowerShell)
New-Item -ItemType Directory -Force -Path "$env:USERPROFILE\.codemap\bin"
Invoke-WebRequest -Uri https://github.com/killvxk/CodeMap/releases/latest/download/codegraph-x86_64-windows.exe `
  -OutFile "$env:USERPROFILE\.codemap\bin\codegraph-x86_64-windows.exe"
```

After installation, use the `codegraph` command directly:

```bash
codegraph scan /path/to/project
codegraph status /path/to/project
codegraph query handleLogin --dir /path/to/project
```

### Option 3: Build from Source

Requires Rust toolchain ([rustup.rs](https://rustup.rs)):

```bash
git clone https://github.com/killvxk/CodeMap.git
cd CodeMap/rust-cli
cargo build --release
# Binary at: target/release/codegraph
```

#### GitHub Release Workflow

```bash
# 1. Ensure tests pass
cd rust-cli && cargo test

# 2. Commit, tag, and let CI build & release
cd ..
git add .
git commit -m "release: v0.2.6"
git tag v0.2.6
git push origin main --tags
# GitHub Actions will automatically build for all platforms and create a Release
```

---

## Project Structure

```
CodeMap/
├── .claude-plugin/
│   └── marketplace.json        # Marketplace manifest
├── ccplugin/                   # Plugin root (CLAUDE_PLUGIN_ROOT)
│   ├── .claude-plugin/
│   │   └── plugin.json         #   Plugin manifest
│   ├── commands/               #   Slash commands
│   │   ├── scan.md             #     /codemap:scan
│   │   ├── load.md             #     /codemap:load
│   │   ├── update.md           #     /codemap:update
│   │   ├── query.md            #     /codemap:query
│   │   ├── impact.md           #     /codemap:impact
│   │   └── prompts.md          #     /codemap:prompts
│   ├── skills/                 #   Auto-triggering skill
│   │   └── codemap/SKILL.md    #     Unified entry, smart routing
│   ├── hooks/                  #   Event hooks
│   │   ├── hooks.json          #     SessionStart auto-detect
│   │   └── scripts/
│   │       └── detect-codemap.sh
│   └── bin/                    #   Binary wrappers
│       ├── codegraph           #     Unix wrapper (auto-discover/download binary)
│       └── codegraph.cmd       #     Windows wrapper
├── rust-cli/                   # Rust CLI source
│   ├── Cargo.toml
│   ├── src/
│   │   ├── main.rs             #   CLI entry (clap)
│   │   ├── scanner.rs          #   Full scan engine
│   │   ├── graph.rs            #   Graph data structures
│   │   ├── differ.rs           #   Incremental update engine
│   │   ├── query.rs            #   Query engine
│   │   ├── slicer.rs           #   Slice generation
│   │   ├── impact.rs           #   Impact analysis
│   │   ├── path_utils.rs       #   Shared path utilities
│   │   ├── traverser.rs        #   File traversal & language detection
│   │   └── languages/          #   Language adapters (8 languages)
│   └── tests/                  #   Integration tests (127 tests)
├── README.md
└── LICENSE                     # MIT
```

---

## CLI Commands

All commands run via `codegraph <command>` (pre-compiled binary, no Node.js required).

| Command | Description |
|---------|-------------|
| `scan <dir>` | Full AST scan, generates `.codemap/` with graph + slices |
| `status [dir]` | Show graph metadata (files, modules, last scan time) |
| `query <symbol>` | Search for functions, classes, types, variables by name |
| `slice [module]` | Output project overview or a specific module slice as JSON |
| `update [dir]` | Incremental update — re-parse only changed files |
| `impact <target>` | Analyze which modules are affected by changing a target |

### Examples

```bash
# Scan a project
codegraph scan /path/to/project

# Check graph status
codegraph status /path/to/project

# Query a symbol
codegraph query "handleLogin" --dir /path/to/project

# Get module slice with dependencies
codegraph slice auth --with-deps --dir /path/to/project

# Incremental update after code changes
codegraph update /path/to/project

# Impact analysis before refactoring
codegraph impact auth --depth 3 --dir /path/to/project
```

---

## Skills & Commands

When installed as a Claude Code plugin, the following capabilities are available:

### Auto-Triggering

The `codemap` skill auto-activates based on conversation context and intelligently routes to the right operation. A `SessionStart` hook also detects `.codemap/` at session start.

### Slash Commands

You can also invoke manually:

| Command | Description |
|-------|------------|
| `/codemap:scan` | Full scan, generate .codemap/ graph |
| `/codemap:load [target]` | Load graph into context (overview/module/file) |
| `/codemap:update` | Incremental update |
| `/codemap:query <symbol>` | Query symbol definitions and call relations |
| `/codemap:impact <target>` | Analyze change impact |
| `/codemap:prompts` | Inject codemap usage rules into project CLAUDE.md |

### Typical Workflow

```
1. First time:        /codemap:scan        → Generate .codemap/ graph
2. New session:       (auto-detected)      → SessionStart hook prompts to load
3. Load overview:     /codemap:load        → Load overview (~500 tokens)
4. Dive into module:  /codemap:load auth   → Load auth module (~2-5k tokens)
5. After changes:     /codemap:update      → Incremental update
6. Before refactor:   /codemap:impact auth → Check impact scope
7. Inject rules:      /codemap:prompts     → Write usage rules to CLAUDE.md
```

---

## Supported Languages

| Language | Extensions | Extracted Structures |
|----------|-----------|---------------------|
| TypeScript | `.ts`, `.tsx` | Functions, imports, exports, classes, interfaces, type aliases, variables (const/let) |
| JavaScript | `.js`, `.jsx`, `.mjs`, `.cjs` | Functions, imports, exports, classes, variables (const/let) |
| Python | `.py` | Functions (decorated), imports, `__all__` exports, classes, module-level variables |
| Go | `.go` | Functions, methods (with receiver), imports, exported names, structs, type specs, variables (var/const) |
| Rust | `.rs` | Functions, impl methods, use declarations, pub exports (incl. const/static), structs, enums, traits, variables (const/static) |
| Java | `.java` | Methods, constructors, imports, public exports, classes, interfaces, enums, static fields |
| C | `.c`, `.h` | Functions, `#include`, non-static exports, structs, enums, typedefs, global variables |
| C++ | `.cpp`, `.cc`, `.cxx`, `.hpp`, `.hh` | Qualified functions (`Class::method`), includes, classes, structs, namespaces, global variables |

---

## Graph Structure

Scanning produces a `.codemap/` directory inside the target project:

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

---

## Tests

```bash
cd rust-cli
cargo test
# 95 unit tests, all passing
```

## License

[MIT](LICENSE)

---

## Appendix: Token Efficiency Comparison — CodeMap vs Grep vs LSP

When an AI coding assistant needs to understand code structure, different tools consume vastly different amounts of tokens. Using the function `analyze_impact` as an example:

### Approach 1: Grep + Read (Traditional)

| Step | Operation | Tokens |
|---|---|---|
| 1 | `Grep "analyze_impact"` — search entire project | ~300-500 |
| 2 | `Read impact.rs` — read definition (280 lines) | ~1500-2000 |
| 3 | `Read commands/impact.rs` — read caller | ~400-600 |
| 4 | `Read tests/impact_compat.rs` — read test references | ~800-1200 |
| 5 | Additional Grep to confirm coverage | ~300-500 |
| **Total** | **4-5 tool calls** | **~3000-5000** |

### Approach 2: LSP (find-references)

| Step | Operation | Tokens |
|---|---|---|
| 1 | `find-references` returns 11 locations | ~200 |
| 2 | `Read impact.rs` to understand context | ~1500 |
| 3 | `Read commands/impact.rs` to understand context | ~500 |
| 4 | `Read tests/impact_compat.rs` to understand context | ~800 |
| **Total** | **3-4 tool calls** | **~3000** |

> LSP returns **raw positions** (file:line:column). The AI agent still needs to Read each file to understand whether a reference is an import or a function call.

### Approach 3: CodeMap query

| Step | Operation | Tokens |
|---|---|---|
| 1 | `codegraph query analyze_impact` — single query returns everything | ~150-200 |
| **Total** | **1 tool call** | **~150-200** |

Results are pre-categorized:

```
[function] analyze_impact (rust-cli/src/impact.rs:35)
  signature: analyze_impact(graph, target, max_depth)
  module:    rust-cli
  lines:     35-68
  usedAt:                          ← Same-file calls
    rust-cli/src/impact.rs :211 :228 :236 :245 :253 :261 :271 :278
  importedBy:                      ← Cross-file references
    rust-cli/src/commands/impact.rs:5 (use :5 :37)
    rust-cli/tests/impact_compat.rs:1 (use :1 :17 :24 :31 :42 ...)
```

### Summary

| | Grep + Read | LSP | CodeMap |
|---|---|---|---|
| Tokens | ~3000-5000 | ~3000 | ~150-200 |
| Tool calls | 4-5 | 3-4 | 1 |
| Savings | Baseline | ~30% | **~95%** |
| Requires file reads | Yes | Yes | No |
| Pre-categorized | No | No | Yes |
| Requires running service | No | Yes | No |
| Cross-language unified | No | No | Yes |

> **Key Insight:** LSP is designed for humans — click a reference in the IDE, jump to it, and understand context visually (zero tokens). CodeMap is designed for AI agents — returns pre-computed structural relationships so the agent understands call chains without reading files.
