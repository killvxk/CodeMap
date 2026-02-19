/**
 * Incremental update engine for detecting file changes and merging graph updates.
 *
 * Compares file hash maps to identify added/modified/removed files,
 * and provides a function to merge partial updates into the full graph.
 */

import path from 'path';

/**
 * Compare two file hash maps to detect changes.
 *
 * @param {Object<string, string>} oldHashes - Previous file hash map (filePath → hash).
 * @param {Object<string, string>} newHashes - Current file hash map (filePath → hash).
 * @returns {{ added: string[], modified: string[], removed: string[], unchanged: string[] }}
 */
export function detectChangedFiles(oldHashes, newHashes) {
  const added = [];
  const modified = [];
  const removed = [];
  const unchanged = [];

  const oldPaths = new Set(Object.keys(oldHashes));
  const newPaths = new Set(Object.keys(newHashes));

  // Check new files against old
  for (const filePath of newPaths) {
    if (!oldPaths.has(filePath)) {
      added.push(filePath);
    } else if (newHashes[filePath] !== oldHashes[filePath]) {
      modified.push(filePath);
    } else {
      unchanged.push(filePath);
    }
  }

  // Find removed files
  for (const filePath of oldPaths) {
    if (!newPaths.has(filePath)) {
      removed.push(filePath);
    }
  }

  return {
    added: added.sort(),
    modified: modified.sort(),
    removed: removed.sort(),
    unchanged: unchanged.sort(),
  };
}

/**
 * Merge updated and removed files into the graph.
 *
 * Mutates the graph in place:
 * - Removes entries for deleted files (from graph.files and from their module's file lists).
 * - Adds/updates entries for changed files.
 * - Removes empty modules.
 * - Recalculates summary (totalFiles, totalFunctions, totalClasses, languages).
 *
 * @param {object} graph - The full code graph (mutated in place).
 * @param {Object<string, object>} updatedFiles - Map of filePath → file data objects to add/update.
 * @param {string[]} removedFiles - Array of file paths to remove from the graph.
 */
export function mergeGraphUpdate(graph, updatedFiles, removedFiles) {
  // Step 1: Remove deleted files
  for (const filePath of removedFiles) {
    const fileData = graph.files[filePath];
    if (fileData) {
      // Remove from module's file list
      const mod = graph.modules[fileData.module];
      if (mod) {
        mod.files = mod.files.filter(f => f !== filePath);
      }
      delete graph.files[filePath];
    }
  }

  // Step 2: Add/update changed files
  for (const [filePath, fileData] of Object.entries(updatedFiles)) {
    // If file already exists and its module changed, remove from old module
    const existing = graph.files[filePath];
    if (existing && existing.module !== fileData.module) {
      const oldMod = graph.modules[existing.module];
      if (oldMod) {
        oldMod.files = oldMod.files.filter(f => f !== filePath);
      }
    }

    // Ensure the target module exists
    if (!graph.modules[fileData.module]) {
      graph.modules[fileData.module] = {
        files: [],
        dependsOn: [],
        dependedBy: [],
      };
    }

    // Add file to module if not already present
    const targetMod = graph.modules[fileData.module];
    if (!targetMod.files.includes(filePath)) {
      targetMod.files.push(filePath);
    }

    // Update file data in the graph
    graph.files[filePath] = fileData;
  }

  // Step 3: Remove empty modules
  for (const [modName, modData] of Object.entries(graph.modules)) {
    if (modData.files.length === 0) {
      delete graph.modules[modName];
    }
  }

  // Step 4: Recalculate summary and rebuild module dependencies
  recalculateSummary(graph);
  rebuildDependencies(graph);
}

/**
 * Recalculate the graph summary from current file and module data.
 *
 * @param {object} graph - The full code graph (mutated in place).
 */
function recalculateSummary(graph) {
  let totalFiles = 0;
  let totalFunctions = 0;
  let totalClasses = 0;
  const languages = {};

  for (const [, fileData] of Object.entries(graph.files)) {
    totalFiles++;
    totalFunctions += fileData.functions ? fileData.functions.length : 0;
    totalClasses += fileData.classes ? fileData.classes.length : 0;
    if (fileData.language) {
      languages[fileData.language] = (languages[fileData.language] || 0) + 1;
    }
  }

  graph.summary.totalFiles = totalFiles;
  graph.summary.totalFunctions = totalFunctions;
  graph.summary.totalClasses = totalClasses;
  graph.summary.languages = languages;
  graph.summary.modules = Object.keys(graph.modules).sort();
  graph.summary.entryPoints = Object.entries(graph.files)
    .filter(([, f]) => f.isEntryPoint)
    .map(([relPath]) => relPath)
    .sort();
  if (graph.config) {
    graph.config.languages = Object.keys(languages);
  }
}

/**
 * Rebuild module-level dependsOn / dependedBy from file-level import data.
 *
 * Iterates every file in the graph, resolves each relative import to a target
 * module via a normalised-path lookup (mirroring scanner.js logic), and
 * overwrites the dependency arrays on every module.
 *
 * @param {object} graph - The full code graph (mutated in place).
 */
function rebuildDependencies(graph) {
  // Build normalised relative-path → moduleName lookup (O(1) resolution)
  const pathLookup = new Map();
  for (const [relPath, fileData] of Object.entries(graph.files)) {
    const norm = relPath.replace(/\\/g, '/');
    pathLookup.set(norm, fileData.module);
    const withoutExt = norm.replace(/\.[^/.]+$/, '');
    if (!pathLookup.has(withoutExt)) {
      pathLookup.set(withoutExt, fileData.module);
    }
  }

  // Collect dependencies using Sets
  const depSets = {};
  for (const modName of Object.keys(graph.modules)) {
    depSets[modName] = { dependsOn: new Set(), dependedBy: new Set() };
  }

  for (const [relPath, fileData] of Object.entries(graph.files)) {
    const moduleName = fileData.module;
    if (!depSets[moduleName]) continue;

    for (const imp of fileData.imports) {
      if (imp.isExternal) continue;
      if (!imp.source.startsWith('.')) continue;

      const importerDir = path.posix.dirname(relPath.replace(/\\/g, '/'));
      const resolved = path.posix.normalize(importerDir + '/' + imp.source);

      let targetModule = pathLookup.get(resolved);
      if (!targetModule) {
        targetModule = pathLookup.get(resolved + '/index');
      }

      if (targetModule && targetModule !== moduleName && depSets[targetModule]) {
        depSets[moduleName].dependsOn.add(targetModule);
        depSets[targetModule].dependedBy.add(moduleName);
      }
    }
  }

  // Apply rebuilt dependencies to graph
  for (const [modName, deps] of Object.entries(depSets)) {
    if (graph.modules[modName]) {
      graph.modules[modName].dependsOn = [...deps.dependsOn].sort();
      graph.modules[modName].dependedBy = [...deps.dependedBy].sort();
    }
  }
}
