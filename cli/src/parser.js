import fs from 'fs/promises';
import path from 'path';
import { fileURLToPath } from 'url';
import { createRequire } from 'module';
import { TypeScriptAdapter } from './languages/typescript.js';
import { GoAdapter } from './languages/go.js';

// web-tree-sitter uses CommonJS; import via createRequire for ESM compat.
const require = createRequire(import.meta.url);
const Parser = require('web-tree-sitter');

// ── Singleton state ──────────────────────────────────────────────────────────

let initialised = false;

/** Loaded Language objects keyed by grammar name (e.g. "typescript"). */
const loadedLanguages = new Map();

/** Language adapter instances keyed by language id. */
const adapters = new Map([
  ['typescript', new TypeScriptAdapter()],
  ['javascript', new TypeScriptAdapter()], // TS grammar is a JS superset
  ['go', new GoAdapter()],
  ['python', new PythonAdapter()],
  ['rust', new RustAdapter()],
  ['java', new JavaAdapter()],
]);

// ── WASM resolution ──────────────────────────────────────────────────────────

/**
 * Resolve the WASM directory that contains pre-built grammar files.
 * Looks for the `tree-sitter-wasms` package first, which ships all the
 * commonly used grammars.
 */
function resolveWasmDir() {
  try {
    // tree-sitter-wasms main entry may not resolve (it points at native
    // bindings that don't exist).  Resolve its package.json instead, which
    // is always present, and derive the out/ directory from there.
    const pkgJson = require.resolve('tree-sitter-wasms/package.json');
    return path.join(path.dirname(pkgJson), 'out');
  } catch {
    throw new Error(
      'Could not resolve tree-sitter-wasms package. Run: npm install tree-sitter-wasms',
    );
  }
}

/**
 * Map from language id used in this project to the grammar WASM file name
 * shipped by tree-sitter-wasms.
 */
const GRAMMAR_FILE = {
  typescript: 'tree-sitter-typescript.wasm',
  javascript: 'tree-sitter-javascript.wasm',
  python: 'tree-sitter-python.wasm',
  go: 'tree-sitter-go.wasm',
  rust: 'tree-sitter-rust.wasm',
  java: 'tree-sitter-java.wasm',
  c: 'tree-sitter-c.wasm',
  cpp: 'tree-sitter-cpp.wasm',
};

// ── Public API ───────────────────────────────────────────────────────────────

/**
 * Initialise the tree-sitter WASM runtime.
 * Must be called once before any `parseFile` call.
 */
export async function initParser() {
  if (initialised) return;
  await Parser.init();
  initialised = true;
}

/**
 * Load (and cache) a tree-sitter Language grammar.
 *
 * @param {string} language - e.g. "typescript"
 * @returns {Promise<import('web-tree-sitter').Language>}
 */
async function loadLanguage(language) {
  if (loadedLanguages.has(language)) {
    return loadedLanguages.get(language);
  }

  const grammarFile = GRAMMAR_FILE[language];
  if (!grammarFile) {
    throw new Error(`No grammar mapping for language "${language}"`);
  }

  const wasmDir = resolveWasmDir();
  const wasmPath = path.join(wasmDir, grammarFile);

  // Verify the file actually exists – give a clear error otherwise.
  try {
    await fs.access(wasmPath);
  } catch {
    throw new Error(
      `Grammar WASM not found at ${wasmPath}. ` +
      `Ensure tree-sitter-wasms is installed.`,
    );
  }

  const lang = await Parser.Language.load(wasmPath);
  loadedLanguages.set(language, lang);
  return lang;
}

/**
 * Parse a source file and extract structural information.
 *
 * @param {string} filePath - Absolute path to the source file.
 * @param {string} language - Language identifier (e.g. "typescript").
 * @returns {Promise<{
 *   functions: Array<{name: string, signature: string, startLine: number, endLine: number}>,
 *   imports: Array<{source: string, symbols: string[], isExternal: boolean}>,
 *   exports: string[],
 *   classes: Array<{name: string, startLine: number, endLine: number}>,
 *   types: Array<{name: string, kind: string, startLine: number, endLine: number}>,
 *   lines: number,
 * }>}
 */
export async function parseFile(filePath, language) {
  if (!initialised) {
    throw new Error('Parser not initialised. Call initParser() first.');
  }

  const adapter = adapters.get(language);
  if (!adapter) {
    throw new Error(`No adapter registered for language "${language}"`);
  }

  const lang = await loadLanguage(language);
  const parser = new Parser();
  parser.setLanguage(lang);

  const sourceCode = await fs.readFile(filePath, 'utf-8');
  const tree = parser.parse(sourceCode);

  const result = {
    functions: adapter.extractFunctions(tree, sourceCode),
    imports: adapter.extractImports(tree, sourceCode),
    exports: adapter.extractExports(tree, sourceCode),
    classes: adapter.extractClasses(tree, sourceCode),
    types: adapter.extractTypes(tree, sourceCode),
    lines: sourceCode.split('\n').length,
  };

  // Clean up WASM resources.
  tree.delete();
  parser.delete();

  return result;
}
