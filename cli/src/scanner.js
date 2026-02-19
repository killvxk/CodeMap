import path from 'path';
import fs from 'fs/promises';
import { traverseFiles, detectLanguage } from './traverser.js';
import { initParser, parseFile } from './parser.js';
import { createEmptyGraph, computeFileHash } from './graph.js';

/** Root-level directories to skip when detecting module names. */
const COMMON_ROOT_DIRS = new Set(['src', 'lib', 'app', 'source', 'packages']);

/** File basenames (without extension) that mark an entry point. */
const ENTRY_POINT_PATTERNS = new Set([
  'main', 'index', 'server', 'app', 'entry', 'bootstrap',
]);

/**
 * Detect a module name from a file's path relative to the project root.
 *
 * Strategy:
 *   - Take the relative path from rootDir to the file.
 *   - Split into segments and drop the filename.
 *   - Skip leading segments that match COMMON_ROOT_DIRS.
 *   - The first remaining directory segment is the module name.
 *   - If no directory remains (file is directly inside a root dir), return '_root'.
 *
 * @param {string} filePath  Absolute path to the file.
 * @param {string} rootDir   Absolute project root directory.
 * @returns {string} Module name.
 */
export function detectModuleName(filePath, rootDir) {
  const rel = path.relative(rootDir, filePath).replace(/\\/g, '/');
  const segments = rel.split('/');
  // Drop the filename
  segments.pop();

  // Skip leading common root dirs
  while (segments.length > 0 && COMMON_ROOT_DIRS.has(segments[0])) {
    segments.shift();
  }

  return segments.length > 0 ? segments[0] : '_root';
}

/**
 * Scan an entire project: traverse files, parse each, build the code graph.
 *
 * @param {string} rootDir   Absolute project root directory.
 * @param {object} [options] Optional overrides.
 * @param {string[]} [options.exclude] Additional glob patterns to exclude.
 * @returns {Promise<object>} The graph data structure.
 */
export async function scanProject(rootDir, options = {}) {
  const projectName = path.basename(rootDir);
  const graph = createEmptyGraph(projectName, rootDir);

  // Ensure parser is ready
  await initParser();

  // Step 1: Traverse to find all source files
  const files = await traverseFiles(rootDir, { exclude: options.exclude || [] });

  // Index: absolute path (normalised with forward slashes) → parsed data + module
  const fileIndex = new Map();
  const languageCounts = {};
  let totalFunctions = 0;
  let totalClasses = 0;
  const moduleSet = new Set();

  // Step 2: Parse each file and collect metadata
  for (const absPath of files) {
    const language = detectLanguage(absPath);
    if (!language) continue;

    let parsed;
    try {
      parsed = await parseFile(absPath, language);
    } catch {
      // Skip files whose language has no adapter (e.g. Python, Go without adapters)
      continue;
    }

    const moduleName = detectModuleName(absPath, rootDir);
    moduleSet.add(moduleName);

    // Language stats
    languageCounts[language] = (languageCounts[language] || 0) + 1;
    totalFunctions += parsed.functions.length;
    totalClasses += parsed.classes.length;

    const relPath = path.relative(rootDir, absPath).replace(/\\/g, '/');
    const content = await fs.readFile(absPath, 'utf-8');
    const hash = computeFileHash(content);

    fileIndex.set(absPath, {
      relPath,
      language,
      moduleName,
      parsed,
      hash,
    });
  }

  // Step 3: Build file entries in the graph and resolve cross-module dependencies
  const modules = {};
  for (const mod of moduleSet) {
    modules[mod] = {
      files: [],
      dependsOn: new Set(),
      dependedBy: new Set(),
    };
  }

  for (const [absPath, info] of fileIndex) {
    const { relPath, language, moduleName, parsed, hash } = info;

    // Detect entry points
    const baseName = path.basename(absPath, path.extname(absPath)).toLowerCase();
    const isEntryPoint = ENTRY_POINT_PATTERNS.has(baseName);

    // Store file data in graph
    graph.files[relPath] = {
      language,
      module: moduleName,
      hash,
      lines: parsed.lines,
      functions: parsed.functions,
      classes: parsed.classes,
      types: parsed.types,
      imports: parsed.imports,
      exports: parsed.exports,
      isEntryPoint,
    };

    // Track file in its module
    modules[moduleName].files.push(relPath);

    // Step 4: Resolve imports to detect cross-module dependencies
    for (const imp of parsed.imports) {
      if (imp.isExternal) continue;

      const resolvedModule = resolveImportModule(absPath, imp.source, rootDir, fileIndex);
      if (resolvedModule && resolvedModule !== moduleName) {
        modules[moduleName].dependsOn.add(resolvedModule);
        if (modules[resolvedModule]) {
          modules[resolvedModule].dependedBy.add(moduleName);
        }
      }
    }
  }

  // Step 5: Populate graph.modules (convert Sets → arrays)
  for (const [modName, modData] of Object.entries(modules)) {
    graph.modules[modName] = {
      files: modData.files,
      dependsOn: [...modData.dependsOn].sort(),
      dependedBy: [...modData.dependedBy].sort(),
    };
  }

  // Step 6: Build summary
  graph.summary.totalFiles = fileIndex.size;
  graph.summary.totalFunctions = totalFunctions;
  graph.summary.totalClasses = totalClasses;
  graph.summary.languages = languageCounts;
  graph.summary.modules = [...moduleSet].sort();
  graph.summary.entryPoints = Object.entries(graph.files)
    .filter(([, f]) => f.isEntryPoint)
    .map(([relPath]) => relPath);
  graph.config.languages = Object.keys(languageCounts);

  return graph;
}

/**
 * Given a file's absolute path and a relative import source string, resolve
 * which module that import targets.
 *
 * @param {string} importerPath  Absolute path of the file containing the import.
 * @param {string} importSource  The raw import source (e.g. '../auth/login').
 * @param {string} rootDir       Project root directory.
 * @param {Map} fileIndex        Map of absPath → { moduleName, ... }.
 * @returns {string|null} The target module name, or null if unresolved.
 */
function resolveImportModule(importerPath, importSource, rootDir, fileIndex) {
  // Only resolve relative imports
  if (!importSource.startsWith('.')) return null;

  const importerDir = path.dirname(importerPath);
  const resolved = path.resolve(importerDir, importSource);
  const resolvedNorm = resolved.replace(/\\/g, '/');

  // Try to find a matching file in the index.
  // The import may omit the extension, so check if any indexed file starts
  // with the resolved path (after normalisation).
  for (const [absPath, info] of fileIndex) {
    const absNorm = absPath.replace(/\\/g, '/');
    // Exact match, or match with extension appended
    if (absNorm === resolvedNorm || absNorm.startsWith(resolvedNorm + '.') || absNorm.startsWith(resolvedNorm + '/index.')) {
      return info.moduleName;
    }
  }

  return null;
}
