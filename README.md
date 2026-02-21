# CodeMap

基于 AST 的代码图谱映射插件，适用于 [Claude Code](https://docs.anthropic.com/en/docs/claude-code)。扫描代码库一次，持久化结构图谱，后续会话加载紧凑切片即可恢复上下文——相比重新读取全部源码，节省约 95% token。

AST-based code graph mapping plugin for [Claude Code](https://docs.anthropic.com/en/docs/claude-code). Scan your codebase once, persist a structural graph, and load compact slices in future sessions — saving ~95% tokens compared to re-reading all source files.

## 特性 / Features

- **AST 解析 / AST Parsing** — 使用 tree-sitter 原生绑定进行精确的结构分析，非正则猜测 / Uses tree-sitter native bindings for accurate structural analysis, no regex guessing
- **多语言支持 / Multi-Language** — TypeScript, JavaScript, Python, Go, Rust, Java, C, C++
- **智能切片 / Smart Slicing** — 项目概览 (~500 tokens) + 按模块切片 (~2-5k tokens)，替代全量源码 (~200k+) / Project overview (~500 tokens) + per-module slices (~2-5k tokens) instead of full source (~200k+)
- **增量更新 / Incremental Updates** — 基于文件哈希比较检测变更，仅重新解析修改的文件 / File hash comparison detects changes; only re-parses modified files
- **影响分析 / Impact Analysis** — 重构前查看哪些模块会受影响 / See what's affected before you refactor
- **自动触发 / Auto-Triggering** — Skill 根据对话上下文自动激活 / Skills activate automatically based on your conversation context

---

## 安装 / Installation

### 前置条件 / Prerequisites

- **Claude Code** CLI ([安装指南 / Install Guide](https://docs.anthropic.com/en/docs/claude-code))

### 方式一：作为 Claude Code 插件安装（推荐）/ Install as Claude Code Plugin (Recommended)

#### 1. 克隆仓库 / Clone the repository

```bash
git clone https://github.com/killvxk/CodeMap.git
cd CodeMap
```

#### 2. 验证 CLI 可用 / Verify CLI works

预编译二进制已包含在 `ccplugin/bin/` 目录中，无需额外安装依赖：

The pre-compiled binary is included in `ccplugin/bin/` — no additional dependencies required:

```bash
# Linux / macOS
ccplugin/bin/codegraph-linux --version    # Linux x64
ccplugin/bin/codegraph-macos --version    # macOS (Intel/Apple Silicon)

# Windows (Git Bash / PowerShell)
ccplugin/bin/codegraph-windows.exe --version
```

> 如果 `ccplugin/bin/` 中没有适合你平台的二进制，可以从 [GitHub Releases](https://github.com/killvxk/CodeMap/releases) 下载，或参考下方"从源码构建"说明。
>
> If no binary matches your platform in `ccplugin/bin/`, download from [GitHub Releases](https://github.com/killvxk/CodeMap/releases) or see "Build from Source" below.

#### 3. 安装为 Claude Code 插件 / Install as Claude Code plugin

在 Claude Code 对话中执行以下命令（注意：这是 Claude Code 内部的斜杠命令，不是终端命令）：

Run the following commands inside a Claude Code session (these are slash commands, not terminal commands):

**方式 A：从本地目录安装（开发/个人使用推荐）/ Option A: Install from local directory**

```
/plugin marketplace add /absolute/path/to/CodeMap
/plugin install codemap@codemap-plugins
```

**方式 B：从 GitHub 安装 / Option B: Install from GitHub**

```
/plugin marketplace add killvxk/CodeMap
/plugin install codemap@codemap-plugins
```

安装后**重启 Claude Code** 使插件生效。

After installation, **restart Claude Code** for the plugin to take effect.

> **原理 / How it works:** Claude Code 读取根目录的 `.claude-plugin/marketplace.json`，其中 `"source": "./ccplugin"` 指向插件目录。然后从 `ccplugin/.claude-plugin/plugin.json` 加载插件清单，自动发现 `ccplugin/commands/` 下的斜杠命令、`ccplugin/skills/` 下的 skill、以及 `ccplugin/hooks/` 下的事件钩子。
>
> Claude Code reads `.claude-plugin/marketplace.json` at the repo root, where `"source": "./ccplugin"` points to the plugin directory. It then loads `ccplugin/.claude-plugin/plugin.json` and auto-discovers commands in `ccplugin/commands/`, skills in `ccplugin/skills/`, and hooks in `ccplugin/hooks/`.

#### 4. 验证插件已安装 / Verify plugin installed

重启 Claude Code 后，输入 / After restarting Claude Code, type:

```
/codemap:codemap-scan
```

如果插件正确安装，该命令会触发代码扫描流程。

If the plugin is installed correctly, this command will trigger the code scan workflow.

#### 卸载 / Uninstall

```
/plugin uninstall codemap@codemap-plugins
```

### 方式二：下载预编译二进制 / Download Pre-compiled Binary

从 [GitHub Releases](https://github.com/killvxk/CodeMap/releases) 下载适合你平台的二进制文件：

Download the binary for your platform from [GitHub Releases](https://github.com/killvxk/CodeMap/releases):

```bash
# Linux x64
curl -L https://github.com/killvxk/CodeMap/releases/latest/download/codegraph-linux -o codegraph
chmod +x codegraph
sudo mv codegraph /usr/local/bin/

# macOS (Intel / Apple Silicon — 通用二进制)
curl -L https://github.com/killvxk/CodeMap/releases/latest/download/codegraph-macos -o codegraph
chmod +x codegraph
sudo mv codegraph /usr/local/bin/

# Windows (PowerShell)
Invoke-WebRequest -Uri https://github.com/killvxk/CodeMap/releases/latest/download/codegraph-windows.exe -OutFile codegraph.exe
```

安装后可以直接使用 `codegraph` 命令：

After installation, use the `codegraph` command directly:

```bash
codegraph scan /path/to/project
codegraph status /path/to/project
codegraph query handleLogin --dir /path/to/project
```

### 方式三：从源码构建 / Build from Source

需要 Rust 工具链（[rustup.rs](https://rustup.rs)）：

Requires Rust toolchain ([rustup.rs](https://rustup.rs)):

```bash
git clone https://github.com/killvxk/CodeMap.git
cd CodeMap/rust-cli
cargo build --release
# 二进制输出到 / Binary at: target/release/codegraph
```

#### GitHub Release 发布流程 / GitHub Release Workflow

```bash
# 1. 确保测试通过 / Ensure tests pass
cd rust-cli && cargo test

# 2. 交叉编译所有平台 / Cross-compile for all platforms
cargo build --release --target x86_64-unknown-linux-gnu
cargo build --release --target aarch64-apple-darwin
cargo build --release --target x86_64-pc-windows-gnu

# 3. 提交并打 tag / Commit and tag
cd ..
git add .
git commit -m "release: v0.2.0"
git tag v0.2.0
git push origin main --tags

# 4. 在 GitHub 创建 Release，上传二进制 / Create GitHub Release and upload binaries
gh release create v0.2.0 \
  ccplugin/bin/codegraph-linux \
  ccplugin/bin/codegraph-macos \
  ccplugin/bin/codegraph-windows.exe \
  --title "CodeMap v0.2.0" \
  --generate-notes
```

---

## 项目结构 / Project Structure

```
CodeMap/
├── .claude-plugin/
│   └── marketplace.json        # 插件市场清单 / Marketplace manifest
├── ccplugin/                   # 插件根目录 (CLAUDE_PLUGIN_ROOT)
│   ├── .claude-plugin/
│   │   └── plugin.json         #   插件清单 / Plugin manifest
│   ├── commands/               #   斜杠命令 / Slash commands
│   │   ├── scan.md             #     /codemap:codemap-scan
│   │   ├── load.md             #     /codemap:codemap-load
│   │   ├── update.md           #     /codemap:codemap-update
│   │   ├── query.md            #     /codemap:codemap-query
│   │   └── impact.md           #     /codemap:codemap-impact
│   ├── skills/                 #   自动触发 Skill / Auto-triggering skill
│   │   └── codemap/SKILL.md    #     统一入口，智能路由 / Unified entry, smart routing
│   ├── hooks/                  #   事件钩子 / Event hooks
│   │   ├── hooks.json          #     SessionStart 自动检测
│   │   └── scripts/
│   │       └── detect-codemap.sh
│   └── bin/                    #   预编译二进制 / Pre-compiled binaries
│       ├── codegraph-linux     #     Linux x64
│       ├── codegraph-macos     #     macOS (Intel + Apple Silicon)
│       └── codegraph-windows.exe #   Windows x64
├── rust-cli/                   # Rust CLI 源码 / Rust CLI source
│   ├── Cargo.toml
│   ├── src/
│   │   ├── main.rs             #   CLI 入口（clap）
│   │   ├── scanner.rs          #   全量扫描引擎
│   │   ├── graph.rs            #   图谱数据结构
│   │   ├── differ.rs           #   增量更新引擎
│   │   ├── query.rs            #   查询引擎
│   │   ├── slicer.rs           #   切片生成
│   │   ├── impact.rs           #   影响分析
│   │   ├── path_utils.rs       #   共享路径工具函数
│   │   ├── traverser.rs        #   文件遍历与语言检测
│   │   └── languages/          #   语言适配器 (8 种)
│   └── tests/                  #   集成测试 (286 tests)
├── README.md
└── LICENSE                     # MIT
```

---

## CLI 命令 / CLI Commands

所有命令通过 `codegraph <command>` 运行（预编译二进制，无需 Node.js）。

All commands run via `codegraph <command>` (pre-compiled binary, no Node.js required).

| 命令 / Command | 描述 / Description |
|---------|-------------|
| `scan <dir>` | 全量 AST 扫描，生成 `.codemap/` 图谱和切片 / Full AST scan, generates `.codemap/` with graph + slices |
| `status [dir]` | 显示图谱元信息（文件数、模块、上次扫描时间）/ Show graph metadata (files, modules, last scan time) |
| `query <symbol>` | 按名称搜索函数、类、类型 / Search for functions, classes, types by name |
| `slice [module]` | 输出项目概览或指定模块切片（JSON）/ Output project overview or a specific module slice as JSON |
| `update [dir]` | 增量更新——仅重新解析变更的文件 / Incremental update — re-parse only changed files |
| `impact <target>` | 分析修改目标会影响哪些模块 / Analyze which modules are affected by changing a target |

### 示例 / Examples

```bash
# 扫描项目 / Scan a project
codegraph scan /path/to/project

# 检查图谱状态 / Check graph status
codegraph status /path/to/project

# 查询符号 / Query a symbol
codegraph query "handleLogin" --dir /path/to/project

# 获取模块切片（含依赖）/ Get module slice with dependencies
codegraph slice auth --with-deps --dir /path/to/project

# 增量更新 / Incremental update after code changes
codegraph update /path/to/project

# 影响分析 / Impact analysis before refactoring
codegraph impact auth --depth 3 --dir /path/to/project
```

---

## Skills & Commands

作为 Claude Code 插件安装后，提供以下能力：

When installed as a Claude Code plugin, the following capabilities are available:

### 自动触发 / Auto-Triggering

`codemap` skill 会根据对话上下文自动激活，智能判断该执行哪个操作。同时 `SessionStart` hook 会在每次会话开始时自动检测 `.codemap/` 是否存在并提示。

The `codemap` skill auto-activates based on conversation context and intelligently routes to the right operation. A `SessionStart` hook also detects `.codemap/` at session start.

### 斜杠命令 / Slash Commands

也可以手动调用：/ You can also invoke manually:

| 命令 / Command | 描述 / Description |
|-------|------------|
| `/codemap:codemap-scan` | 全量扫描项目，生成 .codemap/ 图谱 / Full scan, generate .codemap/ graph |
| `/codemap:codemap-load [target]` | 加载图谱到上下文（概览/模块/文件）/ Load graph into context |
| `/codemap:codemap-update` | 增量更新图谱 / Incremental update |
| `/codemap:codemap-query <symbol>` | 查询符号定义和调用关系 / Query symbol definitions and call relations |
| `/codemap:codemap-impact <target>` | 分析变更影响范围 / Analyze change impact |

### 典型工作流 / Typical Workflow

```
1. 首次使用 / First time:     /codemap:codemap-scan       → 生成 .codemap/ 图谱
2. 新会话开始 / New session:   (自动检测 / auto-detected)  → SessionStart hook 提示加载
3. 加载概览 / Load overview:   /codemap:codemap-load       → 加载概览 (~500 tokens)
4. 深入模块 / Dive into module: /codemap:codemap-load auth → 加载 auth 模块 (~2-5k tokens)
5. 代码修改后 / After changes: /codemap:codemap-update     → 增量更新图谱
6. 重构前 / Before refactor:   /codemap:codemap-impact auth → 查看影响范围
```

---

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

---

## 图谱结构 / Graph Structure

扫描后在目标项目内生成 `.codemap/` 目录：

Scanning produces a `.codemap/` directory inside the target project:

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

---

## 测试 / Tests

```bash
cd rust-cli
cargo test
# 286 tests, all passing
```

## 许可证 / License

[MIT](LICENSE)
