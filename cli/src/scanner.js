import path from 'path';
import fs from 'fs/promises';
import { traverseFiles, detectLanguage } from './traverser.js';
import { initParser, parseFile } from './parser.js';
import { createEmptyGraph, computeFileHash, isEntryPoint } from './graph.js';

/** Root-level directories to skip when detecting module names. */
const COMMON_ROOT_DIRS = new Set(['src', 'lib', 'app', 'source', 'packages']);

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

  // Detect whether the project contains C++ files, so we can treat .h as C++
  const hasCppFiles = files.some(f => {
    const ext = path.extname(f).toLowerCase();
    return ['.cpp', '.cc', '.cxx', '.hpp', '.hh'].includes(ext);
  });

  // Index: absolute path (normalised with forward slashes) → parsed data + module
  const fileIndex = new Map();
  const languageCounts = {};
  let totalFunctions = 0;
  let totalClasses = 0;
  const moduleSet = new Set();

  // Step 2: Parse each file and collect metadata (single read per file)
  for (const absPath of files) {
    let language = detectLanguage(absPath);
    if (!language) continue;

    // Reclassify .h files as C++ when the project contains C++ sources
    if (language === 'c' && hasCppFiles && path.extname(absPath).toLowerCase() === '.h') {
      language = 'cpp';
    }

    let content;
    try {
      content = await fs.readFile(absPath, 'utf-8');
    } catch {
      continue;
    }

    const hash = computeFileHash(content);

    let parsed;
    try {
      parsed = await parseFile(absPath, language, content);
    } catch {
      continue;
    }

    const moduleName = detectModuleName(absPath, rootDir);
    moduleSet.add(moduleName);

    // Language stats
    languageCounts[language] = (languageCounts[language] || 0) + 1;
    totalFunctions += parsed.functions.length;
    totalClasses += parsed.classes.length;

    const relPath = path.relative(rootDir, absPath).replace(/\\/g, '/');

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

  // Build a normalised path → moduleName lookup for O(1) import resolution
  const pathLookup = new Map();
  for (const [absPath, info] of fileIndex) {
    const norm = absPath.replace(/\\/g, '/');
    pathLookup.set(norm, info.moduleName);
    // Also index without extension for extensionless import resolution
    const withoutExt = norm.replace(/\.[^/.]+$/, '');
    if (!pathLookup.has(withoutExt)) {
      pathLookup.set(withoutExt, info.moduleName);
    }
  }

  for (const [absPath, info] of fileIndex) {
    const { relPath, language, moduleName, parsed, hash } = info;

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
      isEntryPoint: isEntryPoint(absPath),
    };

    // Track file in its module
    modules[moduleName].files.push(relPath);

    // Step 4: Resolve imports to detect cross-module dependencies
    for (const imp of parsed.imports) {
      if (imp.isExternal) continue;

      const resolvedModule = resolveImportModule(absPath, imp.source, pathLookup, moduleName);
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
 * which module that import targets using the pre-built path lookup.
 *
 * @param {string} importerPath  Absolute path of the file containing the import.
 * @param {string} importSource  The raw import source (e.g. '../auth/login').
 * @param {Map} pathLookup       Map of normalised path → moduleName.
 * @param {string} fallback      Fallback module name (the importer's own module).
 * @returns {string|null} The target module name, or null if unresolved.
 */
function resolveImportModule(importerPath, importSource, pathLookup, fallback) {
  // Only resolve relative imports
  if (!importSource.startsWith('.')) return null;

  const importerDir = path.dirname(importerPath);
  const resolved = path.resolve(importerDir, importSource).replace(/\\/g, '/');

  // Direct match (with extension already included)
  if (pathLookup.has(resolved)) {
    return pathLookup.get(resolved);
  }

  // Match without extension (e.g. import './utils' → './utils.ts')
  // The pathLookup already indexes paths without extensions
  // Try index file resolution: './auth' → './auth/index'
  const indexPath = resolved + '/index';
  if (pathLookup.has(indexPath)) {
    return pathLookup.get(indexPath);
  }

  return null;
}
