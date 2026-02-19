# CodeMap CLI Rust 重写计划

## 背景

当前 CLI 基于 Node.js + tree-sitter WASM，功能已验证（84 测试全部通过），但存在以下痛点：
- 用户安装插件后需要额外执行 `cd ccplugin/cli && npm install`
- 依赖 Node.js >= 18 运行时
- tree-sitter 走 WASM 间接层，性能非最优
- `node_modules/` 体积大（~30MB+）

## 目标

将 CLI 工具从 Node.js 重写为 Rust，提供预编译二进制文件，实现零依赖安装。

## 方案对比

| 维度 | Node.js (当前) | Rust (目标) |
|------|--------------|------------|
| 安装步骤 | clone → npm install → plugin add | clone → plugin add（零配置） |
| 运行时依赖 | Node.js >= 18 | 无 |
| tree-sitter | WASM 间接调用 | 原生 C 绑定（tree-sitter 本身是 C/Rust） |
| 分发体积 | ~30MB node_modules | 单个二进制 ~5-10MB |
| 跨平台 | 靠 Node 抹平 | 需交叉编译，GitHub Actions 自动化 |
| 扫描性能 | 可用，中等 | 预期提升 3-5x |

## 架构设计

### 目录结构变更

```
ccplugin/
├── bin/
│   ├── codegraph-x86_64-linux         # Linux x64
│   ├── codegraph-aarch64-linux        # Linux ARM64
│   ├── codegraph-x86_64-darwin        # macOS x64
│   ├── codegraph-aarch64-darwin       # macOS ARM64 (Apple Silicon)
│   └── codegraph-x86_64-windows.exe   # Windows x64
├── skills/                             # 不变
└── .claude-plugin/plugin.json          # 不变
```

### SKILL.md 路径变更

从：
```bash
node "${CLAUDE_PLUGIN_ROOT}/cli/bin/codegraph.js" scan .
```

改为（需要平台检测脚本或直接引用）：
```bash
"${CLAUDE_PLUGIN_ROOT}/bin/codegraph" scan .
```

> 需要一个 wrapper 脚本或在 SKILL 中根据平台选择正确的二进制名。

### Rust 项目结构

```
rust-cli/                   # 新建 Rust 项目（与 ccplugin 同级或独立仓库）
├── Cargo.toml
├── build.rs                # 编译时链接 tree-sitter 语法
├── src/
│   ├── main.rs             # CLI 入口（clap）
│   ├── scanner.rs          # 全量扫描引擎（移植自 scanner.js）
│   ├── parser.rs           # tree-sitter 原生解析（移植自 parser.js）
│   ├── graph.rs            # 图谱数据结构（移植自 graph.js）
│   ├── differ.rs           # 增量更新引擎（移植自 differ.js）
│   ├── query.rs            # 查询引擎（移植自 query.js）
│   ├── slicer.rs           # 切片生成（移植自 slicer.js）
│   ├── impact.rs           # 影响分析（移植自 impact.js）
│   ├── traverser.rs        # 文件遍历与语言检测（移植自 traverser.js）
│   └── languages/
│       ├── mod.rs           # 语言适配器 trait
│       ├── typescript.rs
│       ├── python.rs
│       ├── go.rs
│       ├── rust_lang.rs
│       ├── java.rs
│       ├── c.rs
│       └── cpp.rs
└── tests/                  # 移植现有 84 个测试作为验收标准
```

### 关键依赖

```toml
[dependencies]
tree-sitter = "0.24"
tree-sitter-typescript = "0.23"
tree-sitter-javascript = "0.23"
tree-sitter-python = "0.23"
tree-sitter-go = "0.23"
tree-sitter-rust = "0.23"
tree-sitter-java = "0.23"
tree-sitter-c = "0.23"
tree-sitter-cpp = "0.23"
clap = { version = "4", features = ["derive"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
walkdir = "2"
sha2 = "0.10"
ignore = "0.4"           # .gitignore 支持
```

## 实施步骤

### 阶段 1：Rust 项目脚手架
- 初始化 Cargo 项目
- 配置 clap CLI 框架，注册所有子命令
- 配置 tree-sitter 原生绑定，验证所有 8 种语言语法加载
- 实现文件遍历和语言检测（移植 traverser.js）

### 阶段 2：核心引擎移植
- 移植 parser.rs（tree-sitter 原生调用替代 WASM）
- 移植 graph.rs（数据结构，保持 JSON 格式兼容）
- 移植 scanner.rs（全量扫描）
- 移植 slicer.rs（切片生成）

### 阶段 3：语言适配器
- 定义 `LanguageAdapter` trait
- 逐个移植 8 种语言适配器
- 确保 AST 节点查询逻辑与 JS 版本输出一致

### 阶段 4：高级功能
- 移植 differ.rs（增量更新 + rebuildDependencies）
- 移植 query.rs（符号查询）
- 移植 impact.rs（影响分析）

### 阶段 5：测试与兼容性
- 移植现有 84 个测试用例作为验收标准
- 验证输出 JSON 格式与 Node.js 版本完全兼容（.codemap/ 目录格式不变）
- 对同一个 sample-project 做交叉验证：Node.js 扫描 vs Rust 扫描，输出应一致

### 阶段 6：CI/CD 与分发
- GitHub Actions 配置交叉编译（使用 cross 或 cargo-zigbuild）
- 目标平台：x86_64-linux, aarch64-linux, x86_64-darwin, aarch64-darwin, x86_64-windows
- Release 时自动编译并上传二进制到 GitHub Release
- 更新 SKILL.md 路径，添加平台检测 wrapper 脚本
- 更新 README 安装文档

### 阶段 7：清理
- 移除 ccplugin/cli/ 目录（Node.js 版本）
- 将预编译二进制放入 ccplugin/bin/
- 最终验证插件安装流程

## 兼容性约束

- `.codemap/` 输出目录格式必须与 Node.js 版本完全一致
- graph.json、meta.json、slices/*.json 的 JSON schema 不变
- 现有已生成的 .codemap/ 可被 Rust 版本直接读取和增量更新

## 风险

- tree-sitter 语法版本差异可能导致 AST 节点名称不同，需要逐语言验证
- 交叉编译可能遇到平台特定问题（尤其是 Windows + tree-sitter C 绑定）
- 二进制体积需要关注，strip + LTO 优化

## 状态

- [ ] 阶段 1：Rust 项目脚手架
- [ ] 阶段 2：核心引擎移植
- [ ] 阶段 3：语言适配器
- [ ] 阶段 4：高级功能
- [ ] 阶段 5：测试与兼容性
- [ ] 阶段 6：CI/CD 与分发
- [ ] 阶段 7：清理
