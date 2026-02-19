# CodeMap 实现计划

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 构建 Claude Code 插件，通过 AST 解析生成结构化代码图谱、支持智能切片加载和增量更新，节省 95%+ 的 token 消耗。

**Architecture:** Node.js CLI 工具使用 web-tree-sitter (WASM) 进行 AST 解析，生成 JSON 图谱存储在项目 `.codemap/` 目录下。Claude Code skills 层编排 CLI 调用和上下文注入策略。切片系统按模块拆分图谱，支持按需加载。

**Tech Stack:** Node.js 20+, web-tree-sitter (WASM), commander.js, fast-glob, simple-git, vitest

**参考设计文档:** `docs/plans/2026-02-20-codemap-design.md`

---

## Task 1: 项目脚手架

**Files:**
- Create: `cli/package.json`
- Create: `cli/bin/codegraph.js`
- Create: `cli/src/index.js`
- Create: `.claude-plugin/plugin.json`
- Create: `.gitignore`

**Step 1: 初始化 git 仓库**

```bash
cd /e/2026/CodeMap
git init
```

**Step 2: 创建 .gitignore**

Create `.gitignore`:
```
node_modules/
dist/
.codemap/
*.log
.DS_Store
```

**Step 3: 初始化 CLI 项目**

```bash
cd /e/2026/CodeMap
mkdir -p cli/bin cli/src cli/test
cd cli
npm init -y
```

修改 `cli/package.json`:
```json
{
  "name": "codegraph",
  "version": "0.1.0",
  "description": "AST-based code graph generator for Claude Code",
  "type": "module",
  "bin": {
    "codegraph": "./bin/codegraph.js"
  },
  "scripts": {
    "test": "vitest run",
    "test:watch": "vitest"
  },
  "dependencies": {
    "commander": "^13.0.0",
    "fast-glob": "^3.3.0",
    "simple-git": "^3.27.0",
    "web-tree-sitter": "^0.24.0"
  },
  "devDependencies": {
    "vitest": "^3.0.0"
  }
}
```

**Step 4: 创建 CLI 入口**

Create `cli/bin/codegraph.js`:
```javascript
#!/usr/bin/env node
import { createProgram } from '../src/index.js';

const program = createProgram();
program.parse(process.argv);
```

Create `cli/src/index.js`:
```javascript
import { Command } from 'commander';

export function createProgram() {
  const program = new Command();
  program
    .name('codegraph')
    .description('AST-based code graph generator')
    .version('0.1.0');

  // Commands will be registered here
  return program;
}
```

**Step 5: 创建 plugin.json**

Create `.claude-plugin/plugin.json`:
```json
{
  "name": "codemap",
  "description": "Code graph mapping plugin - scan, cache, and load project architecture maps to save tokens",
  "version": "0.1.0",
  "skills": [
    "skills/scan",
    "skills/load",
    "skills/update",
    "skills/query",
    "skills/impact"
  ]
}
```

**Step 6: 安装依赖并验证**

```bash
cd /e/2026/CodeMap/cli
npm install
node bin/codegraph.js --version
```

Expected: `0.1.0`

**Step 7: 提交**

```bash
cd /e/2026/CodeMap
git add .gitignore cli/package.json cli/package-lock.json cli/bin/codegraph.js cli/src/index.js .claude-plugin/plugin.json
git commit -m "chore: scaffold project with CLI entry point and plugin manifest"
```

---

## Task 2: 文件遍历引擎

**Files:**
- Create: `cli/src/traverser.js`
- Create: `cli/test/traverser.test.js`
- Create: `cli/test/fixtures/sample-project/` (test fixture)

**Step 1: 创建测试 fixture**

```bash
mkdir -p /e/2026/CodeMap/cli/test/fixtures/sample-project/src/auth
mkdir -p /e/2026/CodeMap/cli/test/fixtures/sample-project/src/api
mkdir -p /e/2026/CodeMap/cli/test/fixtures/sample-project/node_modules/fake
```

Create `cli/test/fixtures/sample-project/src/auth/login.ts`:
```typescript
import { getUserById } from '../db/users';
import bcrypt from 'bcrypt';

export interface LoginOptions {
  email: string;
  password: string;
}

export async function login(opts: LoginOptions): Promise<string> {
  const user = await getUserById(opts.email);
  const valid = await bcrypt.compare(opts.password, user.hash);
  if (!valid) throw new Error('Invalid credentials');
  return 'token';
}
```

Create `cli/test/fixtures/sample-project/src/api/routes.ts`:
```typescript
import { login } from '../auth/login';

export function handleLogin(req: any, res: any) {
  const token = login(req.body);
  res.json({ token });
}
```

Create `cli/test/fixtures/sample-project/node_modules/fake/index.js`:
```javascript
// should be excluded
```

**Step 2: 写遍历引擎的失败测试**

Create `cli/test/traverser.test.js`:
```javascript
import { describe, it, expect } from 'vitest';
import { traverseFiles } from '../src/traverser.js';
import path from 'path';

const FIXTURE_DIR = path.resolve(import.meta.dirname, 'fixtures/sample-project');

describe('traverseFiles', () => {
  it('should find source files excluding node_modules', async () => {
    const files = await traverseFiles(FIXTURE_DIR);
    const relative = files.map(f => path.relative(FIXTURE_DIR, f).replace(/\\/g, '/'));
    expect(relative).toContain('src/auth/login.ts');
    expect(relative).toContain('src/api/routes.ts');
    expect(relative.some(f => f.includes('node_modules'))).toBe(false);
  });

  it('should filter by language extensions', async () => {
    const files = await traverseFiles(FIXTURE_DIR, { extensions: ['.ts'] });
    expect(files.every(f => f.endsWith('.ts'))).toBe(true);
  });

  it('should respect custom exclude patterns', async () => {
    const files = await traverseFiles(FIXTURE_DIR, { exclude: ['**/api/**'] });
    const relative = files.map(f => path.relative(FIXTURE_DIR, f).replace(/\\/g, '/'));
    expect(relative.some(f => f.includes('api'))).toBe(false);
  });
});
```

**Step 3: 运行测试确认失败**

```bash
cd /e/2026/CodeMap/cli
npx vitest run test/traverser.test.js
```

Expected: FAIL — `traverseFiles` not found

**Step 4: 实现遍历引擎**

Create `cli/src/traverser.js`:
```javascript
import fg from 'fast-glob';
import path from 'path';

const DEFAULT_EXCLUDE = [
  '**/node_modules/**',
  '**/dist/**',
  '**/build/**',
  '**/.git/**',
  '**/vendor/**',
  '**/__pycache__/**',
  '**/target/**',
  '**/.codemap/**',
];

const LANGUAGE_EXTENSIONS = {
  typescript: ['.ts', '.tsx'],
  javascript: ['.js', '.jsx', '.mjs', '.cjs'],
  python: ['.py'],
  go: ['.go'],
  rust: ['.rs'],
  java: ['.java'],
  c: ['.c', '.h'],
  cpp: ['.cpp', '.cc', '.cxx', '.hpp', '.hh'],
};

const ALL_EXTENSIONS = Object.values(LANGUAGE_EXTENSIONS).flat();

export function detectLanguage(filePath) {
  const ext = path.extname(filePath).toLowerCase();
  for (const [lang, exts] of Object.entries(LANGUAGE_EXTENSIONS)) {
    if (exts.includes(ext)) return lang;
  }
  return null;
}

export async function traverseFiles(rootDir, options = {}) {
  const { extensions = ALL_EXTENSIONS, exclude = [] } = options;

  const patterns = extensions.map(ext => `**/*${ext}`);
  const ignorePatterns = [...DEFAULT_EXCLUDE, ...exclude];

  const files = await fg(patterns, {
    cwd: rootDir,
    absolute: true,
    ignore: ignorePatterns,
    dot: false,
  });

  return files.sort();
}

export { LANGUAGE_EXTENSIONS, ALL_EXTENSIONS };
```

**Step 5: 运行测试确认通过**

```bash
cd /e/2026/CodeMap/cli
npx vitest run test/traverser.test.js
```

Expected: PASS (3 tests)

**Step 6: 提交**

```bash
cd /e/2026/CodeMap
git add cli/src/traverser.js cli/test/traverser.test.js cli/test/fixtures/
git commit -m "feat: add file traversal engine with language detection"
```

---

## Task 3: Tree-sitter 集成与语言适配层

**Files:**
- Create: `cli/src/parser.js`
- Create: `cli/src/languages/base.js`
- Create: `cli/src/languages/typescript.js`
- Create: `cli/test/parser.test.js`

**Step 1: 下载 tree-sitter WASM 文件**

```bash
cd /e/2026/CodeMap/cli
mkdir -p grammars
# tree-sitter.wasm 核心运行时由 web-tree-sitter 包提供
# 语言 grammar WASM 需要单独获取
npm install tree-sitter-wasms
```

> 注意: `tree-sitter-wasms` 包含预编译的 WASM grammar 文件。如果该包不可用，可从 tree-sitter GitHub releases 手动下载各语言的 `.wasm` 文件到 `grammars/` 目录。

**Step 2: 写解析器的失败测试**

Create `cli/test/parser.test.js`:
```javascript
import { describe, it, expect, beforeAll } from 'vitest';
import { initParser, parseFile } from '../src/parser.js';
import path from 'path';
import fs from 'fs';

const FIXTURE_DIR = path.resolve(import.meta.dirname, 'fixtures/sample-project');

describe('parser', () => {
  beforeAll(async () => {
    await initParser();
  });

  it('should parse a TypeScript file and extract functions', async () => {
    const filePath = path.join(FIXTURE_DIR, 'src/auth/login.ts');
    const result = await parseFile(filePath, 'typescript');
    expect(result.functions).toBeDefined();
    expect(result.functions.length).toBeGreaterThan(0);
    const loginFn = result.functions.find(f => f.name === 'login');
    expect(loginFn).toBeDefined();
    expect(loginFn.signature).toContain('LoginOptions');
  });

  it('should extract imports', async () => {
    const filePath = path.join(FIXTURE_DIR, 'src/auth/login.ts');
    const result = await parseFile(filePath, 'typescript');
    expect(result.imports).toBeDefined();
    expect(result.imports.length).toBeGreaterThan(0);
    const dbImport = result.imports.find(i => i.source.includes('db/users'));
    expect(dbImport).toBeDefined();
    expect(dbImport.symbols).toContain('getUserById');
  });

  it('should extract exports', async () => {
    const filePath = path.join(FIXTURE_DIR, 'src/auth/login.ts');
    const result = await parseFile(filePath, 'typescript');
    expect(result.exports).toBeDefined();
    expect(result.exports).toContain('login');
    expect(result.exports).toContain('LoginOptions');
  });
});
```

**Step 3: 运行测试确认失败**

```bash
cd /e/2026/CodeMap/cli
npx vitest run test/parser.test.js
```

Expected: FAIL — `parseFile` not found

**Step 4: 实现语言适配器基类**

Create `cli/src/languages/base.js`:
```javascript
/**
 * 语言适配器基类。
 * 每种语言继承此类，实现 extractFunctions / extractImports / extractExports 等方法。
 */
export class LanguageAdapter {
  constructor(language) {
    this.language = language;
  }

  /** @param {import('web-tree-sitter').Tree} tree */
  extractFunctions(tree, sourceCode) {
    throw new Error('Not implemented');
  }

  extractImports(tree, sourceCode) {
    throw new Error('Not implemented');
  }

  extractExports(tree, sourceCode) {
    throw new Error('Not implemented');
  }

  extractClasses(tree, sourceCode) {
    return [];
  }

  extractTypes(tree, sourceCode) {
    return [];
  }
}
```

**Step 5: 实现 TypeScript/JavaScript 适配器**

Create `cli/src/languages/typescript.js`:
```javascript
import { LanguageAdapter } from './base.js';

export class TypeScriptAdapter extends LanguageAdapter {
  constructor() {
    super('typescript');
  }

  extractFunctions(tree, sourceCode) {
    const functions = [];
    const cursor = tree.walk();
    this._visitFunctions(cursor, sourceCode, functions);
    return functions;
  }

  _visitFunctions(cursor, sourceCode, results) {
    const node = cursor.currentNode;

    if (
      node.type === 'function_declaration' ||
      node.type === 'export_statement'
    ) {
      const fnNode = node.type === 'export_statement'
        ? node.children.find(c =>
            c.type === 'function_declaration' ||
            c.type === 'lexical_declaration'
          )
        : node;

      if (fnNode && fnNode.type === 'function_declaration') {
        const nameNode = fnNode.childForFieldName('name');
        const paramsNode = fnNode.childForFieldName('parameters');
        const returnType = fnNode.childForFieldName('return_type');
        if (nameNode) {
          results.push({
            name: nameNode.text,
            signature: this._buildSignature(paramsNode, returnType, sourceCode),
            lines: [fnNode.startPosition.row + 1, fnNode.endPosition.row + 1],
            calls: [],
          });
        }
      }

      // Arrow functions in variable declarations
      if (fnNode && fnNode.type === 'lexical_declaration') {
        for (const declarator of fnNode.children) {
          if (declarator.type === 'variable_declarator') {
            const nameNode = declarator.childForFieldName('name');
            const valueNode = declarator.childForFieldName('value');
            if (nameNode && valueNode && valueNode.type === 'arrow_function') {
              const paramsNode = valueNode.childForFieldName('parameters');
              const returnType = valueNode.childForFieldName('return_type');
              results.push({
                name: nameNode.text,
                signature: this._buildSignature(paramsNode, returnType, sourceCode),
                lines: [declarator.startPosition.row + 1, declarator.endPosition.row + 1],
                calls: [],
              });
            }
          }
        }
      }
    }

    // Recurse into children
    if (cursor.gotoFirstChild()) {
      do {
        this._visitFunctions(cursor, sourceCode, results);
      } while (cursor.gotoNextSibling());
      cursor.gotoParent();
    }
  }

  _buildSignature(paramsNode, returnTypeNode, sourceCode) {
    const params = paramsNode ? paramsNode.text : '()';
    const returnType = returnTypeNode ? returnTypeNode.text : '';
    return returnType ? `${params} => ${returnType.replace(/^:\s*/, '')}` : params;
  }

  extractImports(tree, sourceCode) {
    const imports = [];
    const cursor = tree.walk();
    this._visitImports(cursor, sourceCode, imports);
    return imports;
  }

  _visitImports(cursor, sourceCode, results) {
    const node = cursor.currentNode;

    if (node.type === 'import_statement') {
      const sourceNode = node.children.find(c => c.type === 'string' || c.type === 'string_fragment');
      const source = sourceNode
        ? sourceNode.text.replace(/['"]/g, '')
        : this._extractImportSource(node);

      const symbols = [];
      const importClause = node.children.find(c => c.type === 'import_clause');
      if (importClause) {
        this._extractImportedSymbols(importClause, symbols);
      }

      if (source) {
        results.push({
          source,
          symbols,
          external: !source.startsWith('.') && !source.startsWith('/'),
        });
      }
    }

    if (cursor.gotoFirstChild()) {
      do {
        this._visitImports(cursor, sourceCode, results);
      } while (cursor.gotoNextSibling());
      cursor.gotoParent();
    }
  }

  _extractImportSource(node) {
    // Walk all children to find the string literal for the module path
    for (const child of node.children) {
      if (child.type === 'string') {
        return child.text.replace(/['"]/g, '');
      }
    }
    return null;
  }

  _extractImportedSymbols(node, symbols) {
    if (node.type === 'identifier') {
      symbols.push(node.text);
    }
    if (node.type === 'import_specifier') {
      const nameNode = node.childForFieldName('name') || node.children.find(c => c.type === 'identifier');
      if (nameNode) symbols.push(nameNode.text);
      return;
    }
    for (const child of node.children) {
      this._extractImportedSymbols(child, symbols);
    }
  }

  extractExports(tree, sourceCode) {
    const exports = [];
    const cursor = tree.walk();
    this._visitExports(cursor, sourceCode, exports);
    return exports;
  }

  _visitExports(cursor, sourceCode, results) {
    const node = cursor.currentNode;

    if (node.type === 'export_statement') {
      // export function foo / export class Foo / export interface Foo
      for (const child of node.children) {
        if (child.type === 'function_declaration') {
          const nameNode = child.childForFieldName('name');
          if (nameNode) results.push(nameNode.text);
        } else if (child.type === 'class_declaration' || child.type === 'interface_declaration' || child.type === 'type_alias_declaration') {
          const nameNode = child.childForFieldName('name');
          if (nameNode) results.push(nameNode.text);
        } else if (child.type === 'lexical_declaration') {
          for (const decl of child.children) {
            if (decl.type === 'variable_declarator') {
              const nameNode = decl.childForFieldName('name');
              if (nameNode) results.push(nameNode.text);
            }
          }
        }
      }
    }

    if (cursor.gotoFirstChild()) {
      do {
        this._visitExports(cursor, sourceCode, results);
      } while (cursor.gotoNextSibling());
      cursor.gotoParent();
    }
  }

  extractClasses(tree, sourceCode) {
    const classes = [];
    const cursor = tree.walk();
    this._visitClasses(cursor, sourceCode, classes);
    return classes;
  }

  _visitClasses(cursor, sourceCode, results) {
    const node = cursor.currentNode;

    if (node.type === 'class_declaration') {
      const nameNode = node.childForFieldName('name');
      if (nameNode) {
        results.push({
          name: nameNode.text,
          lines: [node.startPosition.row + 1, node.endPosition.row + 1],
        });
      }
    }

    if (cursor.gotoFirstChild()) {
      do {
        this._visitClasses(cursor, sourceCode, results);
      } while (cursor.gotoNextSibling());
      cursor.gotoParent();
    }
  }

  extractTypes(tree, sourceCode) {
    const types = [];
    const cursor = tree.walk();
    this._visitTypes(cursor, sourceCode, types);
    return types;
  }

  _visitTypes(cursor, sourceCode, results) {
    const node = cursor.currentNode;

    if (node.type === 'interface_declaration' || node.type === 'type_alias_declaration') {
      const nameNode = node.childForFieldName('name');
      if (nameNode) {
        results.push({
          name: nameNode.text,
          kind: node.type === 'interface_declaration' ? 'interface' : 'type',
          lines: [node.startPosition.row + 1, node.endPosition.row + 1],
        });
      }
    }

    // Also check inside export_statement
    if (node.type === 'export_statement') {
      for (const child of node.children) {
        if (child.type === 'interface_declaration' || child.type === 'type_alias_declaration') {
          const nameNode = child.childForFieldName('name');
          if (nameNode) {
            results.push({
              name: nameNode.text,
              kind: child.type === 'interface_declaration' ? 'interface' : 'type',
              lines: [child.startPosition.row + 1, child.endPosition.row + 1],
            });
          }
        }
      }
    }

    if (cursor.gotoFirstChild()) {
      do {
        this._visitTypes(cursor, sourceCode, results);
      } while (cursor.gotoNextSibling());
      cursor.gotoParent();
    }
  }
}
```

**Step 6: 实现解析器（Parser 模块）**

Create `cli/src/parser.js`:
```javascript
import Parser from 'web-tree-sitter';
import path from 'path';
import fs from 'fs/promises';
import { fileURLToPath } from 'url';
import { TypeScriptAdapter } from './languages/typescript.js';

let parserInstance = null;
const languageParsers = {};

const adapters = {
  typescript: new TypeScriptAdapter(),
  javascript: new TypeScriptAdapter(), // TS adapter handles JS too
};

/**
 * 获取 grammar WASM 文件的路径。
 * 优先从 tree-sitter-wasms 包查找，fallback 到本地 grammars/ 目录。
 */
async function findGrammarWasm(language) {
  const grammarNames = {
    typescript: 'tree-sitter-typescript',
    javascript: 'tree-sitter-javascript',
    python: 'tree-sitter-python',
    go: 'tree-sitter-go',
    rust: 'tree-sitter-rust',
  };

  const name = grammarNames[language];
  if (!name) return null;

  // Try tree-sitter-wasms package first
  try {
    const wasmsPath = path.dirname(fileURLToPath(import.meta.resolve('tree-sitter-wasms/package.json')));
    const wasmFile = path.join(wasmsPath, `${name}.wasm`);
    await fs.access(wasmFile);
    return wasmFile;
  } catch {
    // fallback
  }

  // Fallback to local grammars directory
  const __dirname = path.dirname(fileURLToPath(import.meta.url));
  const localPath = path.join(__dirname, '..', 'grammars', `${name}.wasm`);
  try {
    await fs.access(localPath);
    return localPath;
  } catch {
    return null;
  }
}

export async function initParser() {
  if (parserInstance) return;
  await Parser.init();
  parserInstance = new Parser();
}

async function getLanguageParser(language) {
  if (languageParsers[language]) return languageParsers[language];

  const wasmPath = await findGrammarWasm(language);
  if (!wasmPath) {
    throw new Error(`No grammar WASM found for language: ${language}`);
  }

  const lang = await Parser.Language.load(wasmPath);
  languageParsers[language] = lang;
  return lang;
}

export async function parseFile(filePath, language) {
  if (!parserInstance) await initParser();

  const adapter = adapters[language];
  if (!adapter) {
    throw new Error(`No adapter for language: ${language}`);
  }

  const sourceCode = await fs.readFile(filePath, 'utf-8');
  const lang = await getLanguageParser(language);
  parserInstance.setLanguage(lang);

  const tree = parserInstance.parse(sourceCode);

  return {
    functions: adapter.extractFunctions(tree, sourceCode),
    imports: adapter.extractImports(tree, sourceCode),
    exports: adapter.extractExports(tree, sourceCode),
    classes: adapter.extractClasses(tree, sourceCode),
    types: adapter.extractTypes(tree, sourceCode),
    lines: sourceCode.split('\n').length,
  };
}
```

**Step 7: 运行测试**

```bash
cd /e/2026/CodeMap/cli
npx vitest run test/parser.test.js
```

Expected: PASS (3 tests)

> 注意: 如果 `tree-sitter-wasms` 不可用，需要手动下载 TypeScript 的 WASM grammar 并放到 `cli/grammars/` 目录。可从 https://github.com/nicolo-ribaudo/tree-sitter-wasms/releases 获取。

**Step 8: 提交**

```bash
cd /e/2026/CodeMap
git add cli/src/parser.js cli/src/languages/ cli/test/parser.test.js
git commit -m "feat: add tree-sitter parser with TypeScript/JavaScript adapter"
```

---

## Task 4: 图谱数据结构与扫描引擎

**Files:**
- Create: `cli/src/scanner.js`
- Create: `cli/src/graph.js`
- Create: `cli/test/scanner.test.js`

**Step 1: 写图谱数据结构模块**

Create `cli/src/graph.js`:
```javascript
import crypto from 'crypto';
import fs from 'fs/promises';

export function createEmptyGraph(projectName, rootDir) {
  return {
    version: '1.0',
    project: { name: projectName, root: rootDir },
    scannedAt: new Date().toISOString(),
    commitHash: null,
    config: { languages: [], excludePatterns: [] },
    summary: {
      totalFiles: 0,
      totalFunctions: 0,
      totalClasses: 0,
      languages: {},
      modules: [],
      entryPoints: [],
    },
    modules: {},
    files: {},
  };
}

export function computeFileHash(content) {
  return 'sha256:' + crypto.createHash('sha256').update(content).digest('hex').slice(0, 16);
}

export async function saveGraph(outputDir, graph, meta) {
  await fs.mkdir(outputDir, { recursive: true });
  await fs.writeFile(
    `${outputDir}/graph.json`,
    JSON.stringify(graph, null, 2),
    'utf-8'
  );
  await fs.writeFile(
    `${outputDir}/meta.json`,
    JSON.stringify(meta, null, 2),
    'utf-8'
  );
}

export async function loadGraph(outputDir) {
  const graphData = await fs.readFile(`${outputDir}/graph.json`, 'utf-8');
  return JSON.parse(graphData);
}

export async function loadMeta(outputDir) {
  const metaData = await fs.readFile(`${outputDir}/meta.json`, 'utf-8');
  return JSON.parse(metaData);
}
```

**Step 2: 写扫描引擎的失败测试**

Create `cli/test/scanner.test.js`:
```javascript
import { describe, it, expect, beforeAll } from 'vitest';
import { scanProject } from '../src/scanner.js';
import { initParser } from '../src/parser.js';
import path from 'path';

const FIXTURE_DIR = path.resolve(import.meta.dirname, 'fixtures/sample-project');

describe('scanProject', () => {
  beforeAll(async () => {
    await initParser();
  });

  it('should produce a valid graph with summary', async () => {
    const graph = await scanProject(FIXTURE_DIR);
    expect(graph.version).toBe('1.0');
    expect(graph.summary.totalFiles).toBeGreaterThan(0);
    expect(graph.summary.languages).toHaveProperty('typescript');
  });

  it('should detect modules from directory structure', async () => {
    const graph = await scanProject(FIXTURE_DIR);
    expect(graph.summary.modules).toContain('auth');
    expect(graph.summary.modules).toContain('api');
  });

  it('should extract file-level details', async () => {
    const graph = await scanProject(FIXTURE_DIR);
    const loginFile = Object.keys(graph.files).find(f => f.includes('login.ts'));
    expect(loginFile).toBeDefined();
    const fileData = graph.files[loginFile];
    expect(fileData.functions.length).toBeGreaterThan(0);
    expect(fileData.imports.length).toBeGreaterThan(0);
    expect(fileData.exports).toContain('login');
  });

  it('should build module dependency graph', async () => {
    const graph = await scanProject(FIXTURE_DIR);
    // api depends on auth (routes.ts imports from auth/login)
    expect(graph.modules['api']).toBeDefined();
    expect(graph.modules['api'].dependsOn).toContain('auth');
  });
});
```

**Step 3: 运行测试确认失败**

```bash
cd /e/2026/CodeMap/cli
npx vitest run test/scanner.test.js
```

Expected: FAIL — `scanProject` not found

**Step 4: 实现扫描引擎**

Create `cli/src/scanner.js`:
```javascript
import path from 'path';
import fs from 'fs/promises';
import { traverseFiles, detectLanguage } from './traverser.js';
import { parseFile, initParser } from './parser.js';
import { createEmptyGraph, computeFileHash } from './graph.js';

function detectModuleName(filePath, rootDir) {
  const rel = path.relative(rootDir, filePath).replace(/\\/g, '/');
  // e.g., src/auth/login.ts → auth
  const parts = rel.split('/');
  // Skip common root dirs like 'src', 'lib', 'app'
  const skipDirs = new Set(['src', 'lib', 'app', 'source', 'packages']);
  let moduleIdx = 0;
  if (parts.length > 2 && skipDirs.has(parts[0])) {
    moduleIdx = 1;
  }
  return parts.length > moduleIdx + 1 ? parts[moduleIdx] : '_root';
}

function resolveImportToModule(importSource, currentFile, rootDir, fileIndex) {
  if (!importSource.startsWith('.')) return null; // external
  const currentDir = path.dirname(currentFile);
  const resolved = path.resolve(currentDir, importSource).replace(/\\/g, '/');

  // Try to find the actual file (with extensions)
  for (const filePath of Object.keys(fileIndex)) {
    const normalized = filePath.replace(/\\/g, '/');
    if (
      normalized === resolved ||
      normalized.startsWith(resolved + '.') ||
      normalized.startsWith(resolved + '/index.')
    ) {
      return fileIndex[filePath].module;
    }
  }
  return null;
}

export async function scanProject(rootDir, options = {}) {
  await initParser();

  const projectName = path.basename(rootDir);
  const graph = createEmptyGraph(projectName, rootDir);
  const meta = {
    lastScanAt: new Date().toISOString(),
    lastUpdateAt: null,
    commitHash: null,
    scanDuration: 0,
    dirtyFiles: [],
    fileHashes: {},
  };

  const startTime = Date.now();
  const files = await traverseFiles(rootDir, options);

  // Phase 1: Parse all files
  const fileIndex = {};
  for (const filePath of files) {
    const language = detectLanguage(filePath);
    if (!language) continue;

    // Only parse languages we have adapters for
    const supportedLanguages = ['typescript', 'javascript'];
    if (!supportedLanguages.includes(language)) continue;

    const relPath = path.relative(rootDir, filePath).replace(/\\/g, '/');
    const moduleName = detectModuleName(filePath, rootDir);

    try {
      const content = await fs.readFile(filePath, 'utf-8');
      const parsed = await parseFile(filePath, language);
      const hash = computeFileHash(content);

      fileIndex[relPath] = {
        module: moduleName,
        language,
        hash,
        parsed,
        lines: parsed.lines,
      };

      meta.fileHashes[relPath] = hash;
    } catch (err) {
      // Skip files that fail to parse
      console.warn(`Warning: Failed to parse ${relPath}: ${err.message}`);
    }
  }

  // Phase 2: Build module graph
  const moduleMap = {};
  for (const [relPath, fileData] of Object.entries(fileIndex)) {
    const mod = fileData.module;
    if (!moduleMap[mod]) {
      moduleMap[mod] = {
        path: '',
        files: [],
        exports: [],
        dependsOn: new Set(),
        dependedBy: new Set(),
        stats: { files: 0, functions: 0, classes: 0, lines: 0 },
      };
    }
    const m = moduleMap[mod];
    m.files.push(relPath);
    m.stats.files++;
    m.stats.functions += fileData.parsed.functions.length;
    m.stats.classes += fileData.parsed.classes.length;
    m.stats.lines += fileData.lines;

    for (const exp of fileData.parsed.exports) {
      m.exports.push({ name: exp, type: 'function', file: relPath });
    }
  }

  // Phase 3: Resolve cross-module dependencies
  for (const [relPath, fileData] of Object.entries(fileIndex)) {
    const currentModule = fileData.module;
    for (const imp of fileData.parsed.imports) {
      if (imp.external) continue;
      const depModule = resolveImportToModule(imp.source, relPath, rootDir, fileIndex);
      if (depModule && depModule !== currentModule) {
        moduleMap[currentModule].dependsOn.add(depModule);
        if (moduleMap[depModule]) {
          moduleMap[depModule].dependedBy.add(currentModule);
        }
      }
    }
  }

  // Phase 4: Build graph output
  const languageCount = {};
  let totalFunctions = 0;
  let totalClasses = 0;

  for (const [relPath, fileData] of Object.entries(fileIndex)) {
    languageCount[fileData.language] = (languageCount[fileData.language] || 0) + 1;
    totalFunctions += fileData.parsed.functions.length;
    totalClasses += fileData.parsed.classes.length;

    graph.files[relPath] = {
      hash: fileData.hash,
      module: fileData.module,
      language: fileData.language,
      lines: fileData.lines,
      imports: fileData.parsed.imports,
      exports: fileData.parsed.exports,
      functions: fileData.parsed.functions,
      classes: fileData.parsed.classes,
      types: fileData.parsed.types,
    };
  }

  for (const [modName, modData] of Object.entries(moduleMap)) {
    // Determine module path (common prefix of files)
    const firstFile = modData.files[0];
    modData.path = firstFile ? path.dirname(firstFile) + '/' : '';

    graph.modules[modName] = {
      path: modData.path,
      files: modData.files,
      exports: modData.exports,
      dependsOn: [...modData.dependsOn],
      dependedBy: [...modData.dependedBy],
      stats: modData.stats,
    };
  }

  graph.summary = {
    totalFiles: Object.keys(fileIndex).length,
    totalFunctions,
    totalClasses,
    languages: languageCount,
    modules: Object.keys(moduleMap),
    entryPoints: detectEntryPoints(rootDir, fileIndex),
  };

  graph.config.languages = Object.keys(languageCount);
  meta.scanDuration = Date.now() - startTime;

  return graph;
}

function detectEntryPoints(rootDir, fileIndex) {
  const entryPatterns = [
    /^(src\/)?(main|index|app|server)\.(ts|js|mjs)$/,
  ];
  return Object.keys(fileIndex).filter(f =>
    entryPatterns.some(p => p.test(f))
  );
}

export { detectModuleName };
```

**Step 5: 运行测试**

```bash
cd /e/2026/CodeMap/cli
npx vitest run test/scanner.test.js
```

Expected: PASS (4 tests)

**Step 6: 提交**

```bash
cd /e/2026/CodeMap
git add cli/src/scanner.js cli/src/graph.js cli/test/scanner.test.js
git commit -m "feat: add scan engine with module detection and dependency graph"
```

---

## Task 5: 切片生成器

**Files:**
- Create: `cli/src/slicer.js`
- Create: `cli/test/slicer.test.js`

**Step 1: 写切片生成器的失败测试**

Create `cli/test/slicer.test.js`:
```javascript
import { describe, it, expect, beforeAll } from 'vitest';
import { generateSlices, generateOverview } from '../src/slicer.js';
import { scanProject } from '../src/scanner.js';
import { initParser } from '../src/parser.js';
import path from 'path';

const FIXTURE_DIR = path.resolve(import.meta.dirname, 'fixtures/sample-project');

describe('slicer', () => {
  let graph;

  beforeAll(async () => {
    await initParser();
    graph = await scanProject(FIXTURE_DIR);
  });

  it('should generate an overview slice', () => {
    const overview = generateOverview(graph);
    expect(overview.project).toBeDefined();
    expect(overview.modules).toBeDefined();
    expect(overview.entryPoints).toBeDefined();
    // Overview should be compact
    const overviewJson = JSON.stringify(overview);
    expect(overviewJson.length).toBeLessThan(5000);
  });

  it('should generate module slices', () => {
    const slices = generateSlices(graph);
    expect(slices['auth']).toBeDefined();
    expect(slices['auth'].files).toBeDefined();
    expect(slices['auth'].exports).toBeDefined();
  });

  it('should include dependency info in module slices', () => {
    const slices = generateSlices(graph);
    expect(slices['api'].dependsOn).toContain('auth');
  });
});
```

**Step 2: 运行测试确认失败**

```bash
cd /e/2026/CodeMap/cli
npx vitest run test/slicer.test.js
```

Expected: FAIL

**Step 3: 实现切片生成器**

Create `cli/src/slicer.js`:
```javascript
import fs from 'fs/promises';
import path from 'path';

export function generateOverview(graph) {
  return {
    project: graph.project,
    scannedAt: graph.scannedAt,
    commitHash: graph.commitHash,
    summary: graph.summary,
    modules: Object.fromEntries(
      Object.entries(graph.modules).map(([name, mod]) => [
        name,
        {
          path: mod.path,
          fileCount: mod.files.length,
          exports: mod.exports.map(e => e.name),
          dependsOn: mod.dependsOn,
          dependedBy: mod.dependedBy,
          stats: mod.stats,
        },
      ])
    ),
    entryPoints: graph.summary.entryPoints,
  };
}

export function generateSlices(graph) {
  const slices = {};

  for (const [moduleName, moduleData] of Object.entries(graph.modules)) {
    const slice = {
      module: moduleName,
      path: moduleData.path,
      files: {},
      exports: moduleData.exports,
      dependsOn: moduleData.dependsOn,
      dependedBy: moduleData.dependedBy,
      stats: moduleData.stats,
    };

    for (const filePath of moduleData.files) {
      const fileData = graph.files[filePath];
      if (fileData) {
        slice.files[filePath] = {
          language: fileData.language,
          lines: fileData.lines,
          functions: fileData.functions,
          classes: fileData.classes,
          types: fileData.types,
          imports: fileData.imports,
          exports: fileData.exports,
        };
      }
    }

    slices[moduleName] = slice;
  }

  return slices;
}

export function getModuleSliceWithDeps(graph, moduleName) {
  const slices = generateSlices(graph);
  const targetSlice = slices[moduleName];
  if (!targetSlice) return null;

  const result = {
    target: targetSlice,
    dependencies: {},
  };

  // Include overview of dependency modules
  for (const depName of targetSlice.dependsOn) {
    const depModule = graph.modules[depName];
    if (depModule) {
      result.dependencies[depName] = {
        path: depModule.path,
        exports: depModule.exports.map(e => e.name),
        stats: depModule.stats,
      };
    }
  }

  return result;
}

export async function saveSlices(outputDir, graph) {
  const slicesDir = path.join(outputDir, 'slices');
  await fs.mkdir(slicesDir, { recursive: true });

  // Save overview
  const overview = generateOverview(graph);
  await fs.writeFile(
    path.join(slicesDir, '_overview.json'),
    JSON.stringify(overview, null, 2),
    'utf-8'
  );

  // Save module slices
  const slices = generateSlices(graph);
  for (const [moduleName, slice] of Object.entries(slices)) {
    await fs.writeFile(
      path.join(slicesDir, `${moduleName}.json`),
      JSON.stringify(slice, null, 2),
      'utf-8'
    );
  }
}
```

**Step 4: 运行测试**

```bash
cd /e/2026/CodeMap/cli
npx vitest run test/slicer.test.js
```

Expected: PASS (3 tests)

**Step 5: 提交**

```bash
cd /e/2026/CodeMap
git add cli/src/slicer.js cli/test/slicer.test.js
git commit -m "feat: add slice generator with overview and module slicing"
```

---

## Task 6: scan 命令集成

**Files:**
- Modify: `cli/src/index.js`
- Create: `cli/src/commands/scan.js`
- Create: `cli/test/commands/scan.test.js`

**Step 1: 写 scan 命令的集成测试**

Create `cli/test/commands/scan.test.js`:
```javascript
import { describe, it, expect, beforeAll, afterAll } from 'vitest';
import { execSync } from 'child_process';
import path from 'path';
import fs from 'fs';

const FIXTURE_DIR = path.resolve(import.meta.dirname, '../fixtures/sample-project');
const CODEMAP_DIR = path.join(FIXTURE_DIR, '.codemap');
const CLI_BIN = path.resolve(import.meta.dirname, '../../bin/codegraph.js');

describe('codegraph scan command', () => {
  afterAll(() => {
    // Cleanup .codemap in fixture
    fs.rmSync(CODEMAP_DIR, { recursive: true, force: true });
  });

  it('should generate .codemap directory with graph.json', () => {
    execSync(`node "${CLI_BIN}" scan "${FIXTURE_DIR}"`, { encoding: 'utf-8' });
    expect(fs.existsSync(path.join(CODEMAP_DIR, 'graph.json'))).toBe(true);
    expect(fs.existsSync(path.join(CODEMAP_DIR, 'meta.json'))).toBe(true);
  });

  it('should generate slice files', () => {
    expect(fs.existsSync(path.join(CODEMAP_DIR, 'slices', '_overview.json'))).toBe(true);
    expect(fs.existsSync(path.join(CODEMAP_DIR, 'slices', 'auth.json'))).toBe(true);
  });

  it('should output valid JSON in graph.json', () => {
    const graph = JSON.parse(fs.readFileSync(path.join(CODEMAP_DIR, 'graph.json'), 'utf-8'));
    expect(graph.version).toBe('1.0');
    expect(graph.summary.totalFiles).toBeGreaterThan(0);
  });
});
```

**Step 2: 运行测试确认失败**

```bash
cd /e/2026/CodeMap/cli
npx vitest run test/commands/scan.test.js
```

Expected: FAIL

**Step 3: 实现 scan 命令**

Create `cli/src/commands/scan.js`:
```javascript
import path from 'path';
import { scanProject } from '../scanner.js';
import { saveGraph } from '../graph.js';
import { saveSlices } from '../slicer.js';

export function registerScanCommand(program) {
  program
    .command('scan [dir]')
    .description('Scan a project and generate code graph')
    .option('--exclude <patterns...>', 'Additional glob patterns to exclude')
    .action(async (dir, options) => {
      const rootDir = path.resolve(dir || '.');
      const outputDir = path.join(rootDir, '.codemap');

      console.log(`Scanning ${rootDir}...`);
      const startTime = Date.now();

      try {
        const graph = await scanProject(rootDir, {
          exclude: options.exclude,
        });

        // Try to get current git commit hash
        try {
          const { simpleGit } = await import('simple-git');
          const git = simpleGit(rootDir);
          const log = await git.log({ n: 1 });
          if (log.latest) {
            graph.commitHash = log.latest.hash;
          }
        } catch {
          // Not a git repo, that's fine
        }

        const meta = {
          lastScanAt: new Date().toISOString(),
          lastUpdateAt: null,
          commitHash: graph.commitHash,
          scanDuration: Date.now() - startTime,
          dirtyFiles: [],
          fileHashes: Object.fromEntries(
            Object.entries(graph.files).map(([fp, fd]) => [fp, fd.hash])
          ),
        };

        await saveGraph(outputDir, graph, meta);
        await saveSlices(outputDir, graph);

        const duration = Date.now() - startTime;
        console.log(`Done in ${duration}ms.`);
        console.log(`  Files: ${graph.summary.totalFiles}`);
        console.log(`  Functions: ${graph.summary.totalFunctions}`);
        console.log(`  Modules: ${graph.summary.modules.join(', ')}`);
        console.log(`  Output: ${outputDir}/`);
      } catch (err) {
        console.error(`Scan failed: ${err.message}`);
        process.exit(1);
      }
    });
}
```

**Step 4: 注册 scan 命令到 CLI 入口**

Modify `cli/src/index.js`:
```javascript
import { Command } from 'commander';
import { registerScanCommand } from './commands/scan.js';

export function createProgram() {
  const program = new Command();
  program
    .name('codegraph')
    .description('AST-based code graph generator')
    .version('0.1.0');

  registerScanCommand(program);

  return program;
}
```

**Step 5: 运行测试**

```bash
cd /e/2026/CodeMap/cli
npx vitest run test/commands/scan.test.js
```

Expected: PASS (3 tests)

**Step 6: 运行全部测试确保无回归**

```bash
cd /e/2026/CodeMap/cli
npx vitest run
```

Expected: All tests PASS

**Step 7: 提交**

```bash
cd /e/2026/CodeMap
git add cli/src/commands/scan.js cli/src/index.js cli/test/commands/
git commit -m "feat: wire up scan command with graph + slice output"
```

---

## Task 7: query 命令

**Files:**
- Create: `cli/src/commands/query.js`
- Create: `cli/src/query.js`
- Create: `cli/test/query.test.js`

**Step 1: 写 query 引擎的失败测试**

Create `cli/test/query.test.js`:
```javascript
import { describe, it, expect, beforeAll } from 'vitest';
import { querySymbol, queryModule } from '../src/query.js';
import { scanProject } from '../src/scanner.js';
import { initParser } from '../src/parser.js';
import path from 'path';

const FIXTURE_DIR = path.resolve(import.meta.dirname, 'fixtures/sample-project');

describe('query', () => {
  let graph;

  beforeAll(async () => {
    await initParser();
    graph = await scanProject(FIXTURE_DIR);
  });

  it('should find a function by name', () => {
    const results = querySymbol(graph, 'login');
    expect(results.length).toBeGreaterThan(0);
    expect(results[0].name).toBe('login');
    expect(results[0].file).toContain('login.ts');
  });

  it('should return module info', () => {
    const result = queryModule(graph, 'auth');
    expect(result).toBeDefined();
    expect(result.files.length).toBeGreaterThan(0);
    expect(result.exports.length).toBeGreaterThan(0);
  });

  it('should return null for unknown module', () => {
    const result = queryModule(graph, 'nonexistent');
    expect(result).toBeNull();
  });
});
```

**Step 2: 运行测试确认失败**

```bash
cd /e/2026/CodeMap/cli
npx vitest run test/query.test.js
```

**Step 3: 实现 query 引擎**

Create `cli/src/query.js`:
```javascript
export function querySymbol(graph, symbolName, options = {}) {
  const results = [];
  const { type } = options; // 'function' | 'class' | 'type' | undefined

  for (const [filePath, fileData] of Object.entries(graph.files)) {
    if (!type || type === 'function') {
      for (const fn of fileData.functions) {
        if (fn.name === symbolName || fn.name.toLowerCase().includes(symbolName.toLowerCase())) {
          results.push({
            kind: 'function',
            name: fn.name,
            signature: fn.signature,
            file: filePath,
            module: fileData.module,
            lines: fn.lines,
            calls: fn.calls,
            calledBy: fn.calledBy || [],
          });
        }
      }
    }

    if (!type || type === 'class') {
      for (const cls of fileData.classes) {
        if (cls.name === symbolName || cls.name.toLowerCase().includes(symbolName.toLowerCase())) {
          results.push({
            kind: 'class',
            name: cls.name,
            file: filePath,
            module: fileData.module,
            lines: cls.lines,
          });
        }
      }
    }

    if (!type || type === 'type') {
      for (const t of (fileData.types || [])) {
        if (t.name === symbolName || t.name.toLowerCase().includes(symbolName.toLowerCase())) {
          results.push({
            kind: t.kind,
            name: t.name,
            file: filePath,
            module: fileData.module,
            lines: t.lines,
          });
        }
      }
    }
  }

  return results;
}

export function queryModule(graph, moduleName) {
  const mod = graph.modules[moduleName];
  if (!mod) return null;
  return mod;
}

export function queryDependants(graph, moduleName) {
  const mod = graph.modules[moduleName];
  if (!mod) return [];
  return mod.dependedBy || [];
}

export function queryDependencies(graph, moduleName) {
  const mod = graph.modules[moduleName];
  if (!mod) return [];
  return mod.dependsOn || [];
}
```

**Step 4: 实现 query CLI 命令**

Create `cli/src/commands/query.js`:
```javascript
import path from 'path';
import { loadGraph } from '../graph.js';
import { querySymbol, queryModule } from '../query.js';

export function registerQueryCommand(program) {
  program
    .command('query <symbol>')
    .description('Query a symbol in the code graph')
    .option('--type <type>', 'Filter by type: function, class, type')
    .option('--dir <dir>', 'Project root directory', '.')
    .action(async (symbol, options) => {
      const rootDir = path.resolve(options.dir);
      const outputDir = path.join(rootDir, '.codemap');

      try {
        const graph = await loadGraph(outputDir);

        // First try as module name
        const mod = queryModule(graph, symbol);
        if (mod) {
          console.log(`Module: ${symbol}`);
          console.log(`  Path: ${mod.path}`);
          console.log(`  Files: ${mod.files.length}`);
          console.log(`  Exports: ${mod.exports.map(e => e.name).join(', ')}`);
          console.log(`  Depends on: ${mod.dependsOn.join(', ') || 'none'}`);
          console.log(`  Depended by: ${mod.dependedBy.join(', ') || 'none'}`);
          return;
        }

        // Then try as symbol name
        const results = querySymbol(graph, symbol, { type: options.type });
        if (results.length === 0) {
          console.log(`No results found for "${symbol}"`);
          return;
        }

        for (const r of results) {
          console.log(`${r.kind}: ${r.name}`);
          console.log(`  File: ${r.file}:${r.lines[0]}`);
          console.log(`  Module: ${r.module}`);
          if (r.signature) console.log(`  Signature: ${r.signature}`);
          if (r.calls?.length) console.log(`  Calls: ${r.calls.join(', ')}`);
          console.log();
        }
      } catch (err) {
        console.error(`Query failed: ${err.message}`);
        console.error('Have you run "codegraph scan" first?');
        process.exit(1);
      }
    });
}
```

**Step 5: 注册 query 命令**

Modify `cli/src/index.js` — add:
```javascript
import { registerQueryCommand } from './commands/query.js';
```
And register it:
```javascript
registerQueryCommand(program);
```

**Step 6: 运行测试**

```bash
cd /e/2026/CodeMap/cli
npx vitest run
```

Expected: All tests PASS

**Step 7: 提交**

```bash
cd /e/2026/CodeMap
git add cli/src/query.js cli/src/commands/query.js cli/test/query.test.js cli/src/index.js
git commit -m "feat: add query engine for symbol and module lookup"
```

---

## Task 8: 增量更新引擎

**Files:**
- Create: `cli/src/differ.js`
- Create: `cli/src/commands/update.js`
- Create: `cli/test/differ.test.js`

**Step 1: 写 differ 的失败测试**

Create `cli/test/differ.test.js`:
```javascript
import { describe, it, expect } from 'vitest';
import { detectChangedFiles, mergeGraphUpdate } from '../src/differ.js';

describe('differ', () => {
  it('should detect changed files by hash comparison', () => {
    const oldHashes = {
      'src/a.ts': 'sha256:aaa',
      'src/b.ts': 'sha256:bbb',
      'src/c.ts': 'sha256:ccc',
    };
    const newHashes = {
      'src/a.ts': 'sha256:aaa',  // unchanged
      'src/b.ts': 'sha256:bbb2', // modified
      'src/d.ts': 'sha256:ddd',  // added
      // src/c.ts removed
    };

    const changes = detectChangedFiles(oldHashes, newHashes);
    expect(changes.modified).toContain('src/b.ts');
    expect(changes.added).toContain('src/d.ts');
    expect(changes.removed).toContain('src/c.ts');
    expect(changes.unchanged).toContain('src/a.ts');
  });

  it('should merge file updates into existing graph', () => {
    const graph = {
      files: {
        'src/a.ts': { hash: 'old', module: 'root', functions: [] },
        'src/b.ts': { hash: 'old', module: 'root', functions: [] },
      },
      modules: {},
      summary: { totalFiles: 2, totalFunctions: 0, totalClasses: 0, languages: {}, modules: [], entryPoints: [] },
    };

    const updates = {
      'src/b.ts': { hash: 'new', module: 'root', functions: [{ name: 'foo' }], classes: [], types: [], imports: [], exports: [], lines: 10 },
    };
    const removed = ['src/a.ts'];

    mergeGraphUpdate(graph, updates, removed);
    expect(graph.files['src/b.ts'].hash).toBe('new');
    expect(graph.files['src/a.ts']).toBeUndefined();
    expect(graph.summary.totalFiles).toBe(1);
  });
});
```

**Step 2: 运行测试确认失败**

```bash
cd /e/2026/CodeMap/cli
npx vitest run test/differ.test.js
```

**Step 3: 实现 differ**

Create `cli/src/differ.js`:
```javascript
export function detectChangedFiles(oldHashes, newHashes) {
  const result = {
    added: [],
    modified: [],
    removed: [],
    unchanged: [],
  };

  for (const [file, hash] of Object.entries(newHashes)) {
    if (!(file in oldHashes)) {
      result.added.push(file);
    } else if (oldHashes[file] !== hash) {
      result.modified.push(file);
    } else {
      result.unchanged.push(file);
    }
  }

  for (const file of Object.keys(oldHashes)) {
    if (!(file in newHashes)) {
      result.removed.push(file);
    }
  }

  return result;
}

export function mergeGraphUpdate(graph, updatedFiles, removedFiles) {
  // Remove deleted files
  for (const file of removedFiles) {
    delete graph.files[file];
  }

  // Update/add changed files
  for (const [file, data] of Object.entries(updatedFiles)) {
    graph.files[file] = data;
  }

  // Recalculate summary
  let totalFunctions = 0;
  let totalClasses = 0;
  const languages = {};

  for (const [, fileData] of Object.entries(graph.files)) {
    totalFunctions += (fileData.functions || []).length;
    totalClasses += (fileData.classes || []).length;
    languages[fileData.language] = (languages[fileData.language] || 0) + 1;
  }

  graph.summary.totalFiles = Object.keys(graph.files).length;
  graph.summary.totalFunctions = totalFunctions;
  graph.summary.totalClasses = totalClasses;
  graph.summary.languages = languages;
}
```

**Step 4: 实现 update CLI 命令**

Create `cli/src/commands/update.js`:
```javascript
import path from 'path';
import fs from 'fs/promises';
import { loadGraph, loadMeta, saveGraph, computeFileHash } from '../graph.js';
import { saveSlices } from '../slicer.js';
import { traverseFiles, detectLanguage } from '../traverser.js';
import { parseFile, initParser } from '../parser.js';
import { detectChangedFiles, mergeGraphUpdate } from '../differ.js';
import { detectModuleName } from '../scanner.js';

export function registerUpdateCommand(program) {
  program
    .command('update')
    .description('Incrementally update code graph based on file changes')
    .option('--dir <dir>', 'Project root directory', '.')
    .action(async (options) => {
      const rootDir = path.resolve(options.dir);
      const outputDir = path.join(rootDir, '.codemap');

      try {
        await initParser();

        const graph = await loadGraph(outputDir);
        const meta = await loadMeta(outputDir);

        console.log('Checking for changes...');
        const startTime = Date.now();

        // Get current file hashes
        const files = await traverseFiles(rootDir);
        const currentHashes = {};
        const supportedLanguages = ['typescript', 'javascript'];

        for (const filePath of files) {
          const language = detectLanguage(filePath);
          if (!language || !supportedLanguages.includes(language)) continue;
          const relPath = path.relative(rootDir, filePath).replace(/\\/g, '/');
          const content = await fs.readFile(filePath, 'utf-8');
          currentHashes[relPath] = computeFileHash(content);
        }

        const changes = detectChangedFiles(meta.fileHashes, currentHashes);

        if (changes.added.length === 0 && changes.modified.length === 0 && changes.removed.length === 0) {
          console.log('No changes detected.');
          return;
        }

        console.log(`Changes: +${changes.added.length} ~${changes.modified.length} -${changes.removed.length}`);

        // Re-parse changed and added files
        const updatedFiles = {};
        for (const relPath of [...changes.added, ...changes.modified]) {
          const absPath = path.join(rootDir, relPath);
          const language = detectLanguage(absPath);
          if (!language || !supportedLanguages.includes(language)) continue;

          try {
            const parsed = await parseFile(absPath, language);
            updatedFiles[relPath] = {
              hash: currentHashes[relPath],
              module: detectModuleName(absPath, rootDir),
              language,
              lines: parsed.lines,
              imports: parsed.imports,
              exports: parsed.exports,
              functions: parsed.functions,
              classes: parsed.classes,
              types: parsed.types,
            };
          } catch (err) {
            console.warn(`Warning: Failed to parse ${relPath}: ${err.message}`);
          }
        }

        mergeGraphUpdate(graph, updatedFiles, changes.removed);

        // Update metadata
        graph.scannedAt = new Date().toISOString();
        meta.lastUpdateAt = new Date().toISOString();
        meta.fileHashes = currentHashes;
        meta.dirtyFiles = [];
        meta.scanDuration = Date.now() - startTime;

        // Try to get current commit hash
        try {
          const { simpleGit } = await import('simple-git');
          const git = simpleGit(rootDir);
          const log = await git.log({ n: 1 });
          if (log.latest) {
            graph.commitHash = log.latest.hash;
            meta.commitHash = log.latest.hash;
          }
        } catch { /* not a git repo */ }

        await saveGraph(outputDir, graph, meta);
        await saveSlices(outputDir, graph);

        const duration = Date.now() - startTime;
        console.log(`Updated in ${duration}ms.`);
      } catch (err) {
        console.error(`Update failed: ${err.message}`);
        console.error('Have you run "codegraph scan" first?');
        process.exit(1);
      }
    });
}
```

**Step 5: 注册命令**

Modify `cli/src/index.js` — add import and registration for `registerUpdateCommand`.

**Step 6: 运行全部测试**

```bash
cd /e/2026/CodeMap/cli
npx vitest run
```

Expected: All PASS

**Step 7: 提交**

```bash
cd /e/2026/CodeMap
git add cli/src/differ.js cli/src/commands/update.js cli/test/differ.test.js cli/src/index.js
git commit -m "feat: add incremental update engine with file hash diffing"
```

---

## Task 9: impact 分析命令

**Files:**
- Create: `cli/src/impact.js`
- Create: `cli/src/commands/impact.js`
- Create: `cli/test/impact.test.js`

**Step 1: 写失败测试**

Create `cli/test/impact.test.js`:
```javascript
import { describe, it, expect, beforeAll } from 'vitest';
import { analyzeImpact } from '../src/impact.js';
import { scanProject } from '../src/scanner.js';
import { initParser } from '../src/parser.js';
import path from 'path';

const FIXTURE_DIR = path.resolve(import.meta.dirname, 'fixtures/sample-project');

describe('impact analysis', () => {
  let graph;

  beforeAll(async () => {
    await initParser();
    graph = await scanProject(FIXTURE_DIR);
  });

  it('should find dependants of a module', () => {
    const impact = analyzeImpact(graph, 'auth');
    expect(impact.directDependants).toContain('api');
  });

  it('should find impacted files for a specific file', () => {
    const impact = analyzeImpact(graph, 'src/auth/login.ts');
    expect(impact.impactedFiles.length).toBeGreaterThan(0);
  });
});
```

**Step 2: 实现 impact 分析**

Create `cli/src/impact.js`:
```javascript
export function analyzeImpact(graph, target, options = {}) {
  const { depth = 3 } = options;

  // Determine if target is a module or file
  const isModule = !!graph.modules[target];
  const isFile = !!graph.files[target];

  const result = {
    target,
    targetType: isModule ? 'module' : isFile ? 'file' : 'unknown',
    directDependants: [],
    transitiveDependants: [],
    impactedFiles: [],
    impactedModules: [],
  };

  if (isModule) {
    result.directDependants = graph.modules[target].dependedBy || [];
    result.transitiveDependants = findTransitiveDependants(graph, target, depth);
    result.impactedModules = [target, ...result.transitiveDependants];
    result.impactedFiles = result.impactedModules.flatMap(
      mod => graph.modules[mod]?.files || []
    );
  } else if (isFile) {
    const fileData = graph.files[target];
    const moduleName = fileData.module;
    // Files that import from this file
    result.impactedFiles = findFileDependants(graph, target);
    result.directDependants = graph.modules[moduleName]?.dependedBy || [];
    result.impactedModules = [moduleName, ...result.directDependants];
  }

  return result;
}

function findTransitiveDependants(graph, moduleName, maxDepth) {
  const visited = new Set();
  const queue = [{ name: moduleName, depth: 0 }];

  while (queue.length > 0) {
    const { name, depth } = queue.shift();
    if (depth >= maxDepth) continue;

    const mod = graph.modules[name];
    if (!mod) continue;

    for (const dep of mod.dependedBy || []) {
      if (!visited.has(dep)) {
        visited.add(dep);
        queue.push({ name: dep, depth: depth + 1 });
      }
    }
  }

  return [...visited];
}

function findFileDependants(graph, targetFile) {
  const dependants = [];
  for (const [filePath, fileData] of Object.entries(graph.files)) {
    for (const imp of fileData.imports || []) {
      if (imp.external) continue;
      // Check if import source resolves to target file
      if (imp.source && targetFile.includes(imp.source.replace(/^\.\//, ''))) {
        dependants.push(filePath);
        break;
      }
    }
  }
  return dependants;
}
```

**Step 3: 实现 impact CLI 命令**

Create `cli/src/commands/impact.js`:
```javascript
import path from 'path';
import { loadGraph } from '../graph.js';
import { analyzeImpact } from '../impact.js';

export function registerImpactCommand(program) {
  program
    .command('impact <target>')
    .description('Analyze the impact of changing a file or module')
    .option('--depth <n>', 'Max dependency traversal depth', '3')
    .option('--dir <dir>', 'Project root directory', '.')
    .action(async (target, options) => {
      const rootDir = path.resolve(options.dir);
      const outputDir = path.join(rootDir, '.codemap');

      try {
        const graph = await loadGraph(outputDir);
        const impact = analyzeImpact(graph, target, { depth: parseInt(options.depth) });

        console.log(`Impact analysis for: ${target} (${impact.targetType})`);
        console.log(`\nDirect dependants: ${impact.directDependants.join(', ') || 'none'}`);
        if (impact.transitiveDependants.length > 0) {
          console.log(`Transitive dependants: ${impact.transitiveDependants.join(', ')}`);
        }
        console.log(`\nImpacted modules (${impact.impactedModules.length}): ${impact.impactedModules.join(', ')}`);
        console.log(`Impacted files (${impact.impactedFiles.length}):`);
        for (const f of impact.impactedFiles) {
          console.log(`  ${f}`);
        }
      } catch (err) {
        console.error(`Impact analysis failed: ${err.message}`);
        process.exit(1);
      }
    });
}
```

**Step 4: 注册命令，运行测试，提交**

注册 `registerImpactCommand` 到 `cli/src/index.js`。

```bash
cd /e/2026/CodeMap/cli
npx vitest run
```

```bash
cd /e/2026/CodeMap
git add cli/src/impact.js cli/src/commands/impact.js cli/test/impact.test.js cli/src/index.js
git commit -m "feat: add impact analysis with transitive dependency traversal"
```

---

## Task 10: status 和 slice 命令

**Files:**
- Create: `cli/src/commands/status.js`
- Create: `cli/src/commands/slice.js`
- Modify: `cli/src/index.js`

**Step 1: 实现 status 命令**

Create `cli/src/commands/status.js`:
```javascript
import path from 'path';
import fs from 'fs/promises';
import { loadGraph, loadMeta } from '../graph.js';

export function registerStatusCommand(program) {
  program
    .command('status')
    .description('Show code graph status')
    .option('--dir <dir>', 'Project root directory', '.')
    .action(async (options) => {
      const rootDir = path.resolve(options.dir);
      const outputDir = path.join(rootDir, '.codemap');

      try {
        await fs.access(outputDir);
      } catch {
        console.log('No .codemap found. Run "codegraph scan" first.');
        return;
      }

      const graph = await loadGraph(outputDir);
      const meta = await loadMeta(outputDir);

      console.log(`Project: ${graph.project.name}`);
      console.log(`Scanned at: ${graph.scannedAt}`);
      console.log(`Commit: ${graph.commitHash || 'N/A'}`);
      console.log(`Files: ${graph.summary.totalFiles}`);
      console.log(`Functions: ${graph.summary.totalFunctions}`);
      console.log(`Classes: ${graph.summary.totalClasses}`);
      console.log(`Modules: ${graph.summary.modules.join(', ')}`);
      console.log(`Languages: ${JSON.stringify(graph.summary.languages)}`);
      if (meta.lastUpdateAt) {
        console.log(`Last update: ${meta.lastUpdateAt}`);
      }
      if (meta.dirtyFiles.length > 0) {
        console.log(`Dirty files: ${meta.dirtyFiles.join(', ')}`);
      }
    });
}
```

**Step 2: 实现 slice 命令**

Create `cli/src/commands/slice.js`:
```javascript
import path from 'path';
import { loadGraph } from '../graph.js';
import { getModuleSliceWithDeps, generateOverview } from '../slicer.js';

export function registerSliceCommand(program) {
  program
    .command('slice [module]')
    .description('Output a module slice or project overview')
    .option('--with-deps', 'Include dependency module overviews')
    .option('--dir <dir>', 'Project root directory', '.')
    .action(async (module, options) => {
      const rootDir = path.resolve(options.dir);
      const outputDir = path.join(rootDir, '.codemap');

      try {
        const graph = await loadGraph(outputDir);

        if (!module) {
          // Output overview
          const overview = generateOverview(graph);
          console.log(JSON.stringify(overview, null, 2));
          return;
        }

        if (options.withDeps) {
          const result = getModuleSliceWithDeps(graph, module);
          if (!result) {
            console.error(`Module "${module}" not found.`);
            process.exit(1);
          }
          console.log(JSON.stringify(result, null, 2));
        } else {
          const mod = graph.modules[module];
          if (!mod) {
            console.error(`Module "${module}" not found.`);
            process.exit(1);
          }
          // Build slice inline
          const slice = { module, ...mod };
          slice.fileDetails = {};
          for (const f of mod.files) {
            if (graph.files[f]) slice.fileDetails[f] = graph.files[f];
          }
          console.log(JSON.stringify(slice, null, 2));
        }
      } catch (err) {
        console.error(`Slice failed: ${err.message}`);
        process.exit(1);
      }
    });
}
```

**Step 3: 注册所有命令到 index.js**

Modify `cli/src/index.js`:
```javascript
import { Command } from 'commander';
import { registerScanCommand } from './commands/scan.js';
import { registerQueryCommand } from './commands/query.js';
import { registerUpdateCommand } from './commands/update.js';
import { registerImpactCommand } from './commands/impact.js';
import { registerStatusCommand } from './commands/status.js';
import { registerSliceCommand } from './commands/slice.js';

export function createProgram() {
  const program = new Command();
  program
    .name('codegraph')
    .description('AST-based code graph generator')
    .version('0.1.0');

  registerScanCommand(program);
  registerQueryCommand(program);
  registerUpdateCommand(program);
  registerImpactCommand(program);
  registerStatusCommand(program);
  registerSliceCommand(program);

  return program;
}
```

**Step 4: 运行全部测试，提交**

```bash
cd /e/2026/CodeMap/cli
npx vitest run
```

```bash
cd /e/2026/CodeMap
git add cli/src/commands/status.js cli/src/commands/slice.js cli/src/index.js
git commit -m "feat: add status and slice commands for graph inspection"
```

---

## Task 11: Skills — scan skill

**Files:**
- Create: `skills/scan/SKILL.md`

**Step 1: 编写 scan skill**

Create `skills/scan/SKILL.md`:
````markdown
---
name: scan
description: >
  Use when the user wants to scan or index a project codebase, when .codemap/ directory
  does not exist, when starting work on a new project for the first time, or when
  the user says "扫描", "索引", "建立图谱", "scan", "index", "map codebase".
  Also use when the user says they want to understand a project's full architecture
  and no .codemap/ exists yet.
---

# CodeMap Scan — 全量代码图谱扫描

## 执行步骤

1. **检测 .codemap 是否已存在**

```bash
ls -la .codemap/ 2>/dev/null || echo "NO_CODEMAP"
```

如果已存在，提醒用户：图谱已存在。如果只需更新，请使用 `/update`。如果确认要重新全量扫描，继续执行。

2. **执行扫描**

```bash
node "${CLAUDE_PLUGIN_ROOT}/cli/bin/codegraph.js" scan .
```

3. **读取并展示扫描摘要**

读取 `.codemap/slices/_overview.json` 并向用户展示：
- 项目名称和文件总数
- 检测到的模块列表
- 语言分布
- 入口文件

4. **提示后续操作**

告诉用户：
- 使用 `/load <模块名>` 加载特定模块的详细图谱
- 使用 `/query <符号名>` 查询特定函数/类
- 图谱已缓存，下次会话只需 `/load` 即可恢复上下文
````

**Step 2: 提交**

```bash
cd /e/2026/CodeMap
mkdir -p skills/scan
git add skills/scan/SKILL.md
git commit -m "feat: add scan skill for full project scanning"
```

---

## Task 12: Skills — load skill

**Files:**
- Create: `skills/load/SKILL.md`

**Step 1: 编写 load skill**

Create `skills/load/SKILL.md`:
````markdown
---
name: load
description: >
  Use when starting work on a project that has a .codemap/ directory, when the user
  asks about project structure or architecture, when the user wants to understand
  code before making changes, or when beginning any coding task.
  Keywords: 加载图谱, 项目结构, 架构, load, 了解代码, 开始工作, 代码地图,
  查看模块, understand codebase, project overview, code structure.
  Also use proactively at session start if .codemap/ exists.
---

# CodeMap Load — 智能加载代码图谱

## 执行步骤

### 1. 检测图谱是否存在

```bash
ls .codemap/graph.json 2>/dev/null || echo "NO_CODEMAP"
```

如果不存在，建议用户先执行 `/scan`。

### 2. 检查图谱新鲜度

```bash
node "${CLAUDE_PLUGIN_ROOT}/cli/bin/codegraph.js" status
```

如果 commitHash 与当前 HEAD 不一致，建议先执行 `/update`。

### 3. 加载策略

**无参数调用 `/load`** — 加载概览：

读取 `.codemap/slices/_overview.json`，向上下文注入项目概览（~500 token）。

**带模块名 `/load auth`** — 加载模块切片：

```bash
node "${CLAUDE_PLUGIN_ROOT}/cli/bin/codegraph.js" slice auth --with-deps
```

读取输出并注入上下文，包含：
- 目标模块的完整图谱（函数签名、导入导出、类型）
- 依赖模块的概览信息

**带文件路径 `/load src/auth/login.ts`** — 加载文件上下文：

```bash
node "${CLAUDE_PLUGIN_ROOT}/cli/bin/codegraph.js" query login
```

### 4. 智能推断

如果用户描述了任务（如"修改登录功能"），从描述中提取关键词，匹配模块名或符号名，自动加载相关切片。

步骤：
1. 读取 `.codemap/slices/_overview.json` 获取模块列表
2. 匹配用户描述中的关键词与模块名/导出符号
3. 加载匹配到的模块切片
````

**Step 2: 提交**

```bash
cd /e/2026/CodeMap
mkdir -p skills/load
git add skills/load/SKILL.md
git commit -m "feat: add load skill with smart slice selection"
```

---

## Task 13: Skills — update, query, impact skills

**Files:**
- Create: `skills/update/SKILL.md`
- Create: `skills/query/SKILL.md`
- Create: `skills/impact/SKILL.md`

**Step 1: 编写 update skill**

Create `skills/update/SKILL.md`:
````markdown
---
name: update
description: >
  Use after code has been modified, after git commits, when the code graph
  might be outdated, or when the user says "更新图谱", "同步", "refresh",
  "update map", "代码改了".
---

# CodeMap Update — 增量更新图谱

## 执行步骤

1. **执行增量更新**

```bash
node "${CLAUDE_PLUGIN_ROOT}/cli/bin/codegraph.js" update
```

2. **展示变更摘要**

向用户报告：
- 新增/修改/删除的文件数
- 更新耗时
- 受影响的模块

3. **如果需要，重新加载受影响模块的切片**

如果当前上下文已加载了某些模块，且这些模块被更新影响，重新加载其切片。
````

**Step 2: 编写 query skill**

Create `skills/query/SKILL.md`:
````markdown
---
name: query
description: >
  Use when the user asks about a specific function, class, type, or module.
  Keywords: 查找, 查询, 哪里定义, 谁调用了, 在哪个文件, find function,
  where is, who calls, definition of, 函数签名, 调用关系.
---

# CodeMap Query — 符号查询

## 执行步骤

1. **执行查询**

```bash
node "${CLAUDE_PLUGIN_ROOT}/cli/bin/codegraph.js" query "<symbol>" --type <function|class|type>
```

2. **展示结果**

向用户展示：
- 符号定义位置（文件:行号）
- 函数签名
- 调用者和被调用者
- 所属模块

3. **如果需要深入查看，读取源文件的具体行**

根据查询结果的行号范围，使用 Read 工具读取源文件对应的代码段。
````

**Step 3: 编写 impact skill**

Create `skills/impact/SKILL.md`:
````markdown
---
name: impact
description: >
  Use when the user wants to know the impact of changing a file or module,
  when planning refactoring, or when assessing risk of a change.
  Keywords: 影响范围, 影响分析, 改这个会影响, refactor impact, what depends on,
  who uses this, 风险评估, change impact.
---

# CodeMap Impact — 变更影响分析

## 执行步骤

1. **执行影响分析**

```bash
node "${CLAUDE_PLUGIN_ROOT}/cli/bin/codegraph.js" impact "<target>" --depth 3
```

其中 `<target>` 可以是模块名或文件路径。

2. **展示影响范围**

向用户报告：
- 直接依赖方（直接导入此模块/文件的模块）
- 传递依赖方（间接受影响的模块）
- 受影响的文件总数
- 建议重点关注的文件

3. **如果影响范围较大，建议用户考虑分步重构**
````

**Step 4: 提交**

```bash
cd /e/2026/CodeMap
mkdir -p skills/update skills/query skills/impact
git add skills/update/SKILL.md skills/query/SKILL.md skills/impact/SKILL.md
git commit -m "feat: add update, query, and impact skills"
```

---

## Task 14: 端到端集成测试

**Files:**
- Create: `cli/test/e2e/full-workflow.test.js`

**Step 1: 编写端到端测试**

Create `cli/test/e2e/full-workflow.test.js`:
```javascript
import { describe, it, expect, beforeAll, afterAll } from 'vitest';
import { execSync } from 'child_process';
import path from 'path';
import fs from 'fs';

const FIXTURE_DIR = path.resolve(import.meta.dirname, '../fixtures/sample-project');
const CODEMAP_DIR = path.join(FIXTURE_DIR, '.codemap');
const CLI_BIN = path.resolve(import.meta.dirname, '../../bin/codegraph.js');

function run(cmd) {
  return execSync(`node "${CLI_BIN}" ${cmd}`, { encoding: 'utf-8', cwd: FIXTURE_DIR });
}

describe('E2E: full workflow', () => {
  afterAll(() => {
    fs.rmSync(CODEMAP_DIR, { recursive: true, force: true });
  });

  it('scan → status → query → slice', () => {
    // Scan
    const scanOutput = run('scan .');
    expect(scanOutput).toContain('Done');
    expect(fs.existsSync(path.join(CODEMAP_DIR, 'graph.json'))).toBe(true);

    // Status
    const statusOutput = run('status');
    expect(statusOutput).toContain('sample-project');

    // Query
    const queryOutput = run('query login');
    expect(queryOutput).toContain('login');

    // Slice overview
    const overviewOutput = run('slice');
    const overview = JSON.parse(overviewOutput);
    expect(overview.summary).toBeDefined();

    // Slice module
    const sliceOutput = run('slice auth --with-deps');
    const slice = JSON.parse(sliceOutput);
    expect(slice.target).toBeDefined();
  });

  it('update should detect no changes after fresh scan', () => {
    const output = run('update');
    expect(output).toContain('No changes');
  });
});
```

**Step 2: 运行全部测试**

```bash
cd /e/2026/CodeMap/cli
npx vitest run
```

Expected: All PASS

**Step 3: 提交**

```bash
cd /e/2026/CodeMap
git add cli/test/e2e/
git commit -m "test: add end-to-end workflow integration test"
```

---

## Task 15: 最终验证与清理

**Step 1: 运行完整测试套件**

```bash
cd /e/2026/CodeMap/cli
npx vitest run --reporter=verbose
```

Expected: All PASS

**Step 2: 验证 CLI 可执行**

```bash
cd /e/2026/CodeMap/cli
node bin/codegraph.js --help
node bin/codegraph.js scan ../some-test-project  # 用一个真实项目测试
```

**Step 3: 验证插件结构完整**

```bash
ls -la /e/2026/CodeMap/.claude-plugin/plugin.json
ls -la /e/2026/CodeMap/skills/*/SKILL.md
```

应看到: scan, load, update, query, impact 五个 skill

**Step 4: 最终提交**

```bash
cd /e/2026/CodeMap
git add -A
git commit -m "chore: finalize CodeMap plugin v0.1.0"
```

---

## 检查列表状态

- [ ] Task 1: 项目脚手架
- [ ] Task 2: 文件遍历引擎
- [ ] Task 3: Tree-sitter 集成与语言适配层
- [ ] Task 4: 图谱数据结构与扫描引擎
- [ ] Task 5: 切片生成器
- [ ] Task 6: scan 命令集成
- [ ] Task 7: query 命令
- [ ] Task 8: 增量更新引擎
- [ ] Task 9: impact 分析命令
- [ ] Task 10: status 和 slice 命令
- [ ] Task 11: Skills — scan skill
- [ ] Task 12: Skills — load skill
- [ ] Task 13: Skills — update, query, impact skills
- [ ] Task 14: 端到端集成测试
- [ ] Task 15: 最终验证与清理
