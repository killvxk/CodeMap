# CodeMap 插件设计文档

> 日期: 2026-02-20
> 状态: 设计完成，待实现

## 1. 问题与目标

### 问题
每次 Claude Code 会话开始时，需要全量扫描/读取项目代码来理解架构，导致：
- 大量 token 消耗在重复读取上（500 文件项目可消耗 200k+ token）
- 上下文窗口被源码填满，留给实际工作的空间不足
- 跨会话无法复用之前的代码理解

### 目标
构建一套 Claude Code 插件（Skills + CLI 工具），实现：
1. **一次扫描，持久缓存** — 生成结构化代码图谱，存储为 JSON 文件
2. **智能加载，按需注入** — 只加载相关模块的图谱切片，而非全部源码
3. **差异化更新** — 基于 git diff 增量更新，无需重新全量扫描
4. **自动化触发** — 通过 skill description 让 Claude 自动识别并调用

### 量化指标
| 场景 | 当前 token 消耗 | 目标 token 消耗 | 节省 |
|------|----------------|----------------|------|
| 项目全量了解 | 200k+ | 3-5k (概览+切片) | ~97% |
| 修改某模块前的上下文 | 50-100k | 2-5k (模块切片) | ~95% |
| 跨会话恢复上下文 | 重新全量 | 500 token (概览) | ~99% |

---

## 2. 架构设计

### 2.1 总体架构

```
┌─────────────────────────────────────────────────────────┐
│                    Claude Code 会话                       │
│                                                          │
│  用户输入 ──→ Claude 匹配 skill description              │
│               ──→ 调用对应 skill                         │
│               ──→ skill 指导 Claude 调用 CLI             │
│               ──→ CLI 在进程外执行（不消耗 token）        │
│               ──→ skill 指导 Claude 读取结果注入上下文    │
└─────────────────────────────────────────────────────────┘
```

### 2.2 分层职责

| 层 | 职责 | 实现 |
|----|------|------|
| **Skills 层** | 编排 + 自动触发 + 上下文注入策略 | Markdown skill 文件 |
| **CLI 层** | AST 解析 + 图谱生成 + 增量 diff + 切片查询 | Node.js (tree-sitter) |
| **存储层** | 图谱持久化 + 元数据 + 切片缓存 | JSON 文件 (.codemap/) |

### 2.3 插件目录结构

```
codemap-plugin/
├── .claude-plugin/
│   └── plugin.json              ← 插件清单
│
├── skills/
│   ├── scan/SKILL.md            ← 全量扫描技能
│   ├── load/SKILL.md            ← 智能加载技能
│   ├── update/SKILL.md          ← 增量更新技能
│   ├── query/SKILL.md           ← 符号查询技能
│   └── impact/SKILL.md          ← 影响分析技能
│
├── cli/
│   ├── package.json
│   ├── bin/
│   │   └── codegraph.js         ← CLI 入口 (#!/usr/bin/env node)
│   └── src/
│       ├── index.js             ← 命令路由
│       ├── scanner.js           ← AST 扫描引擎
│       ├── differ.js            ← 增量 diff 引擎
│       ├── slicer.js            ← 切片生成/查询
│       ├── query.js             ← 符号/依赖查询
│       ├── languages.js         ← 语言适配层 (tree-sitter grammars)
│       └── utils.js             ← 通用工具
│
└── README.md
```

---

## 3. 数据模型

### 3.1 graph.json — 核心图谱

```jsonc
{
  "version": "1.0",
  "project": {
    "name": "my-app",
    "root": "/absolute/path"
  },
  "scannedAt": "2026-02-20T12:00:00Z",
  "commitHash": "abc123def",
  "config": {
    "languages": ["typescript", "javascript", "python"],
    "excludePatterns": ["node_modules", "dist", ".git", "*.test.*"]
  },
  "summary": {
    "totalFiles": 150,
    "totalFunctions": 820,
    "totalClasses": 45,
    "languages": { "typescript": 120, "javascript": 20, "python": 10 },
    "modules": ["auth", "api", "db", "ui", "config", "utils"],
    "entryPoints": ["src/main.ts", "src/server.ts"]
  },
  "modules": {
    "auth": {
      "path": "src/auth/",
      "files": ["src/auth/login.ts", "src/auth/jwt.ts", "src/auth/middleware.ts"],
      "exports": [
        { "name": "login", "type": "function", "file": "src/auth/login.ts" },
        { "name": "verifyToken", "type": "function", "file": "src/auth/jwt.ts" },
        { "name": "AuthMiddleware", "type": "class", "file": "src/auth/middleware.ts" }
      ],
      "dependsOn": ["db", "config"],
      "dependedBy": ["api"],
      "stats": { "files": 3, "functions": 12, "classes": 2, "lines": 340 }
    }
  },
  "files": {
    "src/auth/login.ts": {
      "hash": "sha256:a1b2c3...",
      "module": "auth",
      "language": "typescript",
      "lines": 120,
      "imports": [
        { "source": "src/db/users.ts", "symbols": ["getUserById", "UserModel"] },
        { "source": "bcrypt", "symbols": ["compare"], "external": true }
      ],
      "exports": ["login", "LoginOptions"],
      "functions": [
        {
          "name": "login",
          "signature": "(email: string, password: string) => Promise<AuthToken>",
          "lines": [15, 58],
          "calls": ["getUserById", "bcrypt.compare", "generateToken"],
          "calledBy": ["handleLoginRoute"]
        },
        {
          "name": "validateCredentials",
          "signature": "(email: string) => boolean",
          "lines": [60, 75],
          "calls": [],
          "calledBy": ["login"]
        }
      ],
      "classes": [],
      "types": [
        { "name": "LoginOptions", "kind": "interface", "lines": [5, 12] }
      ]
    }
  }
}
```

### 3.2 meta.json — 扫描元数据

```jsonc
{
  "lastScanAt": "2026-02-20T12:00:00Z",
  "lastUpdateAt": "2026-02-20T14:30:00Z",
  "commitHash": "abc123def",
  "scanDuration": 3200,  // ms
  "dirtyFiles": [],      // 已知变更但未更新的文件
  "fileHashes": {
    "src/auth/login.ts": "sha256:a1b2c3...",
    "src/auth/jwt.ts": "sha256:d4e5f6..."
  }
}
```

### 3.3 切片格式

切片按模块存储在 `.codemap/slices/` 目录下，每个切片是图谱的一个子集，可独立加载。

```
.codemap/
├── graph.json        ← 完整图谱
├── meta.json         ← 元数据
└── slices/
    ├── _overview.json    ← L0 概览切片 (~500 token)
    ├── auth.json         ← auth 模块切片
    ├── api.json          ← api 模块切片
    └── ...
```

---

## 4. Skills 设计

### 4.1 /scan — 全量扫描

**触发条件**: 新项目首次使用、用户明确要求扫描、.codemap/ 不存在

**流程**:
1. 检测项目根目录和技术栈
2. 调用 `codegraph scan .` 执行 AST 扫描
3. 生成 graph.json + meta.json + 切片文件
4. 输出扫描摘要

### 4.2 /load — 智能加载

**触发条件**: 会话开始时、用户询问项目结构/架构、开始编码任务前

**流程**:
1. 检查 .codemap/ 是否存在，不存在则建议先 /scan
2. 检查图谱新鲜度（commitHash vs 当前 HEAD）
3. 若用户指定模块 → 加载概览 + 目标模块切片 + 依赖模块概览
4. 若未指定 → 只加载概览切片

**智能切片选择逻辑**:
- 解析用户输入中的模块/文件/函数关键词
- 匹配图谱中的模块名和文件路径
- 加载匹配模块 + 其一级依赖模块的概览

### 4.3 /update — 增量更新

**触发条件**: 代码变更后、git commit 后、图谱可能过期时

**流程**:
1. 读取 meta.json 获取上次 commitHash
2. 调用 `codegraph update` (内部 git diff + 增量 AST)
3. 只重新解析变更的文件
4. 合并到现有 graph.json
5. 重新生成受影响的切片
6. 输出变更摘要

### 4.4 /query — 符号查询

**触发条件**: 用户查询某个函数/类/模块的信息

**流程**:
1. 调用 `codegraph query <symbol>`
2. 返回：定义位置、签名、调用者、被调用者、所属模块

### 4.5 /impact — 影响分析

**触发条件**: 用户想知道修改某文件/函数会影响哪些其他部分

**流程**:
1. 调用 `codegraph impact <file-or-symbol>`
2. 遍历依赖图，返回所有直接和间接依赖方
3. 输出影响范围和建议关注点

---

## 5. CLI 工具设计

### 5.1 命令接口

```bash
# 全量扫描
codegraph scan [dir] [--config codemap.config.json]

# 增量更新
codegraph update [--since <commit>]

# 查询符号
codegraph query <symbol> [--type function|class|module]

# 影响分析
codegraph impact <file-or-symbol> [--depth 2]

# 输出切片
codegraph slice <module> [--with-deps]

# 检查图谱状态
codegraph status
```

### 5.2 技术选型

| 组件 | 选择 | 理由 |
|------|------|------|
| AST 解析 | tree-sitter (node-tree-sitter) | 40+ 语言支持，增量解析能力 |
| CLI 框架 | commander.js | 轻量、成熟 |
| 文件哈希 | Node.js crypto (SHA256) | 内置，无依赖 |
| Git 操作 | simple-git | Node.js git 封装，轻量 |
| 文件遍历 | fast-glob | 高性能，支持 gitignore |

### 5.3 语言支持（首批）

| 语言 | tree-sitter grammar | 优先级 |
|------|---------------------|--------|
| TypeScript/JavaScript | tree-sitter-typescript | P0 |
| Python | tree-sitter-python | P0 |
| Go | tree-sitter-go | P1 |
| Rust | tree-sitter-rust | P1 |
| Java | tree-sitter-java | P2 |
| C/C++ | tree-sitter-c / tree-sitter-cpp | P2 |

### 5.4 性能目标

| 指标 | 目标 |
|------|------|
| 首次全量扫描 (500 文件) | < 10s |
| 增量更新 (10 文件变更) | < 1s |
| 图谱加载 (概览切片) | < 100ms |
| graph.json 大小 (500 文件) | < 500KB |

---

## 6. 实现计划

### Phase 1: 基础框架
- [ ] 初始化项目结构（plugin.json + CLI package.json）
- [ ] CLI 命令路由 + scan 命令骨架
- [ ] tree-sitter 集成 + TypeScript/JavaScript 语言适配
- [ ] graph.json 数据结构定义 + 写入

### Phase 2: 核心扫描
- [ ] AST 扫描引擎：文件遍历 + 函数/类/导入导出提取
- [ ] 模块自动检测（目录结构 + package.json 推断）
- [ ] summary 和 modules 生成
- [ ] meta.json + 文件哈希生成
- [ ] scan skill 编写

### Phase 3: 切片与加载
- [ ] 切片生成器：按模块拆分 graph.json
- [ ] 概览切片 (_overview.json) 生成
- [ ] load skill 编写（智能切片选择逻辑）
- [ ] query 命令 + skill 编写

### Phase 4: 增量更新
- [ ] git diff 集成 (simple-git)
- [ ] 增量 AST 解析（只处理变更文件）
- [ ] graph.json 合并逻辑
- [ ] 切片增量重新生成
- [ ] update skill 编写

### Phase 5: 高级功能
- [ ] impact 影响分析（依赖图遍历）
- [ ] Python 语言适配
- [ ] 配置文件支持 (codemap.config.json)
- [ ] impact skill 编写

### Phase 6: 测试与发布
- [ ] 单元测试（scanner, differ, slicer）
- [ ] 集成测试（完整扫描→更新→查询流程）
- [ ] 发布到 npm + skills.sh

---

## 7. 风险与缓解

| 风险 | 缓解 |
|------|------|
| tree-sitter 不同语言的 grammar 一致性 | 抽象语言适配层，每种语言独立实现 |
| 大型 monorepo 的图谱过大 | 支持配置排除目录和深度限制 |
| 模块边界不明确的项目 | 提供配置文件手动指定模块划分 |
| skill description 触发不精确 | 使用丰富的关键词和场景描述 |
