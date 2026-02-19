/**
 * Impact analysis engine for determining the blast radius of changes.
 *
 * Performs BFS through the module dependency graph (via dependedBy edges)
 * to find all transitively impacted modules and their files.
 */

/**
 * Analyze the impact of a change to a module or file.
 *
 * @param {object} graph - The full code graph.
 * @param {string} target - A module name or file path.
 * @param {object} [options] - Optional settings.
 * @param {number} [options.depth=3] - Maximum BFS depth for transitive dependants.
 * @returns {{
 *   targetType: 'module'|'file',
 *   targetModule: string,
 *   directDependants: string[],
 *   transitiveDependants: string[],
 *   impactedModules: string[],
 *   impactedFiles: string[],
 * }}
 */
export function analyzeImpact(graph, target, options = {}) {
  const maxDepth = options.depth != null ? options.depth : 3;

  // Determine if target is a module or a file
  let targetModule;
  let targetType;

  if (graph.modules[target]) {
    targetType = 'module';
    targetModule = target;
  } else {
    // Try to find it as a file path
    const fileData = graph.files[target];
    if (fileData) {
      targetType = 'file';
      targetModule = fileData.module;
    } else {
      // Try partial file path match
      const matchingFile = Object.keys(graph.files).find(f => f.includes(target));
      if (matchingFile) {
        targetType = 'file';
        targetModule = graph.files[matchingFile].module;
      } else {
        // Not found - return empty result
        return {
          targetType: 'module',
          targetModule: target,
          directDependants: [],
          transitiveDependants: [],
          impactedModules: [target],
          impactedFiles: [],
        };
      }
    }
  }

  // Get direct dependants
  const modData = graph.modules[targetModule];
  const directDependants = modData ? [...(modData.dependedBy || [])] : [];

  // BFS through dependedBy to find transitive dependants
  const transitiveDependants = bfsDependants(graph, targetModule, maxDepth);

  // All impacted modules = target + all transitive dependants
  const impactedModules = [targetModule, ...transitiveDependants];

  // All impacted files = files belonging to impacted modules
  const impactedFiles = [];
  for (const modName of impactedModules) {
    const mod = graph.modules[modName];
    if (mod) {
      impactedFiles.push(...mod.files);
    }
  }

  return {
    targetType,
    targetModule,
    directDependants,
    transitiveDependants,
    impactedModules,
    impactedFiles: impactedFiles.sort(),
  };
}

/**
 * BFS through dependedBy edges starting from a module, up to a maximum depth.
 *
 * @param {object} graph - The full code graph.
 * @param {string} startModule - The starting module name.
 * @param {number} maxDepth - Maximum BFS depth.
 * @returns {string[]} Array of all transitively dependent module names (excluding the start).
 */
function bfsDependants(graph, startModule, maxDepth) {
  const visited = new Set([startModule]);
  const result = [];
  let currentLevel = [startModule];

  for (let depth = 0; depth < maxDepth; depth++) {
    const nextLevel = [];

    for (const modName of currentLevel) {
      const mod = graph.modules[modName];
      if (!mod || !mod.dependedBy) continue;

      for (const dep of mod.dependedBy) {
        if (!visited.has(dep)) {
          visited.add(dep);
          result.push(dep);
          nextLevel.push(dep);
        }
      }
    }

    if (nextLevel.length === 0) break;
    currentLevel = nextLevel;
  }

  return result.sort();
}
