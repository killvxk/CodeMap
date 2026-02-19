/**
 * Query engine for symbol and module lookup within the code graph.
 *
 * Provides functions to search for functions, classes, types by name,
 * retrieve module information, and inspect module dependencies.
 */

/**
 * Search all files in the graph for matching functions/classes/types.
 *
 * @param {object} graph - The full code graph from scanProject().
 * @param {string} symbolName - The name to search for (case-sensitive substring match).
 * @param {object} [options] - Optional filters.
 * @param {'function'|'class'|'type'} [options.type] - Restrict search to a specific kind.
 * @returns {Array<{kind: string, name: string, signature?: string, file: string, module: string, lines: {start: number, end: number}, calls: string[], calledBy: string[]}>}
 */
export function querySymbol(graph, symbolName, options = {}) {
  const results = [];
  const typeFilter = options.type || null;

  for (const [filePath, fileData] of Object.entries(graph.files)) {
    const moduleName = fileData.module;

    // Search functions
    if (!typeFilter || typeFilter === 'function') {
      for (const fn of fileData.functions) {
        if (fn.name === symbolName || fn.name.includes(symbolName)) {
          // Imported symbols in the same file (not actual call graph)
          const fileImports = fileData.imports
            .flatMap(imp => imp.symbols || [])
            .filter(s => s !== fn.name);

          // Files/modules that import this symbol
          const importedBy = findCallers(graph, filePath, fn.name);

          results.push({
            kind: 'function',
            name: fn.name,
            signature: fn.signature || null,
            file: filePath,
            module: moduleName,
            lines: { start: fn.startLine, end: fn.endLine },
            fileImports,
            importedBy,
          });
        }
      }
    }

    // Search classes
    if (!typeFilter || typeFilter === 'class') {
      for (const cls of fileData.classes) {
        if (cls.name === symbolName || cls.name.includes(symbolName)) {
          const importedBy = findCallers(graph, filePath, cls.name);
          results.push({
            kind: 'class',
            name: cls.name,
            file: filePath,
            module: moduleName,
            lines: { start: cls.startLine, end: cls.endLine },
            fileImports: [],
            importedBy,
          });
        }
      }
    }

    // Search types
    if (!typeFilter || typeFilter === 'type') {
      for (const tp of fileData.types) {
        if (tp.name === symbolName || tp.name.includes(symbolName)) {
          const importedBy = findCallers(graph, filePath, tp.name);
          results.push({
            kind: 'type',
            name: tp.name,
            file: filePath,
            module: moduleName,
            lines: { start: tp.startLine, end: tp.endLine },
            fileImports: [],
            importedBy,
          });
        }
      }
    }
  }

  return results;
}

/**
 * Find files/modules that import a given symbol from a given file.
 *
 * @param {object} graph - The full code graph.
 * @param {string} sourceFile - The file path that exports the symbol.
 * @param {string} symbolName - The exported symbol name.
 * @returns {string[]} Array of "module:file" strings that reference the symbol.
 */
function findCallers(graph, sourceFile, symbolName) {
  const callers = [];

  for (const [filePath, fileData] of Object.entries(graph.files)) {
    if (filePath === sourceFile) continue;

    for (const imp of fileData.imports) {
      // Check if the import source could resolve to sourceFile
      // and if the imported symbols include our symbol
      if (imp.symbols && imp.symbols.includes(symbolName)) {
        callers.push(`${fileData.module}:${filePath}`);
        break;
      }
    }
  }

  return callers;
}

/**
 * Returns the module data for the given module name, or null if not found.
 *
 * @param {object} graph - The full code graph.
 * @param {string} moduleName - The module name to look up.
 * @returns {object|null} Module data including files, dependsOn, dependedBy, or null.
 */
export function queryModule(graph, moduleName) {
  const modData = graph.modules[moduleName];
  if (!modData) return null;

  return {
    name: moduleName,
    files: modData.files,
    dependsOn: modData.dependsOn,
    dependedBy: modData.dependedBy,
  };
}

/**
 * Returns the dependedBy array for the given module, i.e., which modules depend on it.
 *
 * @param {object} graph - The full code graph.
 * @param {string} moduleName - The module name.
 * @returns {string[]} Array of module names that depend on this module.
 */
export function queryDependants(graph, moduleName) {
  const modData = graph.modules[moduleName];
  if (!modData) return [];
  return modData.dependedBy || [];
}

/**
 * Returns the dependsOn array for the given module, i.e., which modules it depends on.
 *
 * @param {object} graph - The full code graph.
 * @param {string} moduleName - The module name.
 * @returns {string[]} Array of module names this module depends on.
 */
export function queryDependencies(graph, moduleName) {
  const modData = graph.modules[moduleName];
  if (!modData) return [];
  return modData.dependsOn || [];
}
