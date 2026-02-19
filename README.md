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

---

## 安装 / Installation

### 前置条件 / Prerequisites

- **Node.js** >= 18
- **npm** >= 9
- **Claude Code** CLI ([安装指南 / Install Guide](https://docs.anthropic.com/en/docs/claude-code))

### 方式一：作为 Claude Code 插件安装（推荐）/ Install as Claude Code Plugin (Recommended)

#### 1. 克隆仓库 / Clone the repository

```bash
git clone https://github.com/killvxk/CodeMap.git
cd CodeMap
```

#### 2. 安装 CLI 依赖 / Install CLI dependencies

```bash
cd ccplugin/cli
npm install
cd ../..
```

#### 3. 验证 CLI 可用 / Verify CLI works

```bash
node ccplugin/cli/bin/codegraph.js --version
# 输出 / Output: 0.1.0
```

#### 4. 安装为 Claude Code 插件 / Install as Claude Code plugin

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

> **原理 / How it works:** Claude Code 读取根目录的 `.claude-plugin/marketplace.json`，其中 `"source": "./ccplugin"` 指向插件目录。然后从 `ccplugin/.claude-plugin/plugin.json` 加载插件清单，自动发现 `ccplugin/skills/` 下的所有 skill。
>
> Claude Code reads `.claude-plugin/marketplace.json` at the repo root, where `"source": "./ccplugin"` points to the plugin directory. It then loads `ccplugin/.claude-plugin/plugin.json` and auto-discovers all skills under `ccplugin/skills/`.

#### 5. 验证插件已安装 / Verify plugin installed

重启 Claude Code 后，输入 / After restarting Claude Code, type:

```
/scan
```

如果插件正确安装，该 skill 会被识别并触发代码扫描流程。

If the plugin is installed correctly, this skill will be recognized and trigger the code scan workflow.

#### 卸载 / Uninstall

```
/plugin uninstall codemap@codemap-plugins
```

### 方式二：全局安装 CLI / Global CLI Installation

如果你只需要 CLI 工具（不需要 Claude Code 插件集成）：

If you only need the CLI tool (without Claude Code plugin integration):

```bash
git clone https://github.com/killvxk/CodeMap.git
cd CodeMap/ccplugin/cli
npm install
npm link
```

安装后可以直接使用 `codegraph` 命令：

After installation, use the `codegraph` command directly:

```bash
codegraph scan /path/to/project
codegraph status /path/to/project
codegraph query handleLogin --dir /path/to/project
```

### 方式三：构建发布包 / Build Release Distribution

用于将 CLI 打包分发给他人或部署到 CI。

For packaging the CLI to distribute or deploy in CI.

#### 生成 npm tarball / Generate npm tarball

```bash
cd ccplugin/cli
npm pack
# 生成 / Produces: codegraph-0.1.0.tgz
```

#### 从 tarball 安装 / Install from tarball

```bash
npm install -g codegraph-0.1.0.tgz
codegraph --version
```

#### GitHub Release 发布流程 / GitHub Release Workflow

```bash
# 1. 确保测试通过 / Ensure tests pass
cd ccplugin/cli && npm test

# 2. 更新版本号 / Bump version
npm version patch  # 或 minor / major

# 3. 生成发布包 / Generate release package
npm pack

# 4. 提交并打 tag / Commit and tag
cd ../..
git add .
git commit -m "release: v$(node -p "require('./ccplugin/cli/package.json').version")"
git tag "v$(node -p "require('./ccplugin/cli/package.json').version")"
git push origin main --tags

# 5. 在 GitHub 创建 Release，上传 .tgz 文件
# Create a GitHub Release and upload the .tgz file
gh release create "v$(node -p "require('./ccplugin/cli/package.json').version")" \
  ccplugin/cli/codegraph-*.tgz \
  --title "CodeMap v$(node -p "require('./ccplugin/cli/package.json').version")" \
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
│   ├── skills/                 #   Claude Code Skills (自动触发)
│   │   ├── scan/SKILL.md       #     /scan - 全量扫描
│   │   ├── load/SKILL.md       #     /load - 智能加载
│   │   ├── update/SKILL.md     #     /update - 增量更新
│   │   ├── query/SKILL.md      #     /query - 符号查询
│   │   └── impact/SKILL.md     #     /impact - 影响分析
│   └── cli/                    #   CLI 工具
│       ├── bin/codegraph.js    #     入口 / Entry point
│       ├── src/                #     源码 / Source
│       │   ├── index.js        #       Commander 注册
│       │   ├── scanner.js      #       全量扫描引擎
│       │   ├── parser.js       #       tree-sitter WASM 解析
│       │   ├── graph.js        #       图谱数据结构
│       │   ├── differ.js       #       增量更新引擎
│       │   ├── query.js        #       查询引擎
│       │   ├── slicer.js       #       切片生成
│       │   ├── impact.js       #       影响分析
│       │   ├── traverser.js    #       文件遍历与语言检测
│       │   ├── commands/       #       CLI 命令实现
│       │   └── languages/      #       语言适配器 (8 种)
│       ├── test/               #     测试 (84 tests)
│       └── package.json
├── README.md
└── LICENSE                     # MIT
```

---

## CLI 命令 / CLI Commands

所有命令通过 `codegraph <command>` 或 `node ccplugin/cli/bin/codegraph.js <command>` 运行。

All commands run via `codegraph <command>` or `node ccplugin/cli/bin/codegraph.js <command>`.

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

### 典型工作流 / Typical Workflow

```
1. 首次使用 / First time:     /scan           → 生成 .codemap/ 图谱
2. 新会话开始 / New session:   /load           → 加载概览 (~500 tokens)
3. 深入模块 / Dive into module: /load auth     → 加载 auth 模块 (~2-5k tokens)
4. 代码修改后 / After changes: /update         → 增量更新图谱
5. 重构前 / Before refactor:   /impact auth    → 查看影响范围
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
cd ccplugin/cli
npm test
# 84 tests, 14 test suites
```

## 许可证 / License

[MIT](LICENSE)
