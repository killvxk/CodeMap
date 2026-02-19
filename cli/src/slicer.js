import fs from 'fs/promises';
import path from 'path';

/**
 * Generate a compact overview of the project graph.
 *
 * The overview includes project metadata, a summary, simplified module info
 * (path, fileCount, export names, dependsOn, dependedBy, stats), and entry points.
 *
 * @param {object} graph - The full code graph from scanProject().
 * @returns {object} A compact overview object.
 */
export function generateOverview(graph) {
  const modules = Object.entries(graph.modules).map(([modName, modData]) => {
    // Collect all exports across files in this module
    const allExports = [];
    let totalFunctions = 0;
    let totalClasses = 0;
    let totalLines = 0;

    for (const filePath of modData.files) {
      const fileData = graph.files[filePath];
      if (!fileData) continue;
      allExports.push(...fileData.exports);
      totalFunctions += fileData.functions.length;
      totalClasses += fileData.classes.length;
      totalLines += fileData.lines;
    }

    return {
      name: modName,
      path: modData.files.length > 0 ? path.dirname(modData.files[0]) : modName,
      fileCount: modData.files.length,
      exports: [...new Set(allExports)],
      dependsOn: modData.dependsOn,
      dependedBy: modData.dependedBy,
      stats: {
        totalFiles: modData.files.length,
        totalFunctions,
        totalClasses,
        totalLines,
      },
    };
  });

  return {
    project: graph.project,
    scannedAt: graph.scannedAt,
    commitHash: graph.commitHash,
    summary: graph.summary,
    modules,
    entryPoints: graph.summary.entryPoints,
  };
}

/**
 * Generate full slices for every module in the graph.
 *
 * Each slice contains the module name, path, full file details,
 * exports, dependsOn, dependedBy, and stats.
 *
 * @param {object} graph - The full code graph from scanProject().
 * @returns {object} An object keyed by module name, each value being a module slice.
 */
export function generateSlices(graph) {
  const slices = {};

  for (const [modName, modData] of Object.entries(graph.modules)) {
    slices[modName] = buildModuleSlice(graph, modName, modData);
  }

  return slices;
}

/**
 * Build a full slice for a single module.
 *
 * @param {object} graph - The full code graph.
 * @param {string} modName - Module name.
 * @param {object} modData - Module data from graph.modules[modName].
 * @returns {object} The module slice.
 */
export function buildModuleSlice(graph, modName, modData) {
  const files = [];
  const allExports = [];
  let totalFunctions = 0;
  let totalClasses = 0;
  let totalLines = 0;

  for (const filePath of modData.files) {
    const fileData = graph.files[filePath];
    if (!fileData) continue;

    files.push({
      path: filePath,
      language: fileData.language,
      lines: fileData.lines,
      functions: fileData.functions,
      classes: fileData.classes,
      types: fileData.types,
      imports: fileData.imports,
      exports: fileData.exports,
      isEntryPoint: fileData.isEntryPoint,
      hash: fileData.hash,
    });

    allExports.push(...fileData.exports);
    totalFunctions += fileData.functions.length;
    totalClasses += fileData.classes.length;
    totalLines += fileData.lines;
  }

  return {
    module: modName,
    path: modData.files.length > 0 ? path.dirname(modData.files[0]) : modName,
    files,
    exports: [...new Set(allExports)],
    dependsOn: modData.dependsOn,
    dependedBy: modData.dependedBy,
    stats: {
      totalFiles: modData.files.length,
      totalFunctions,
      totalClasses,
      totalLines,
    },
  };
}

/**
 * Get the target module's full slice plus simplified dependency information.
 *
 * For each dependency (dependsOn), a simplified object is included with
 * name, exports, fileCount, and stats.
 *
 * @param {object} graph - The full code graph.
 * @param {string} moduleName - The target module name.
 * @returns {object} The module slice with a `dependencies` array of simplified dep info.
 */
export function getModuleSliceWithDeps(graph, moduleName) {
  const modData = graph.modules[moduleName];
  if (!modData) {
    throw new Error(`Module "${moduleName}" not found in graph`);
  }

  const slice = buildModuleSlice(graph, moduleName, modData);

  // Build simplified dependency info for each dependsOn module
  const dependencies = modData.dependsOn.map(depName => {
    const depData = graph.modules[depName];
    if (!depData) {
      return { name: depName, exports: [], fileCount: 0, stats: {} };
    }

    const depExports = [];
    let totalFunctions = 0;
    let totalClasses = 0;
    let totalLines = 0;

    for (const filePath of depData.files) {
      const fileData = graph.files[filePath];
      if (!fileData) continue;
      depExports.push(...fileData.exports);
      totalFunctions += fileData.functions.length;
      totalClasses += fileData.classes.length;
      totalLines += fileData.lines;
    }

    return {
      name: depName,
      exports: [...new Set(depExports)],
      fileCount: depData.files.length,
      stats: {
        totalFiles: depData.files.length,
        totalFunctions,
        totalClasses,
        totalLines,
      },
    };
  });

  return {
    ...slice,
    dependencies,
  };
}

/**
 * Save the overview and per-module slices as JSON files.
 *
 * Creates `{outputDir}/slices/_overview.json` and `{outputDir}/slices/{moduleName}.json`.
 *
 * @param {string} outputDir - The base output directory (e.g. `.codemap`).
 * @param {object} graph - The full code graph.
 */
export async function saveSlices(outputDir, graph) {
  const slicesDir = path.join(outputDir, 'slices');
  await fs.mkdir(slicesDir, { recursive: true });

  // Save overview
  const overview = generateOverview(graph);
  await fs.writeFile(
    path.join(slicesDir, '_overview.json'),
    JSON.stringify(overview, null, 2),
    'utf-8',
  );

  // Save per-module slices
  const slices = generateSlices(graph);
  for (const [modName, slice] of Object.entries(slices)) {
    await fs.writeFile(
      path.join(slicesDir, `${modName}.json`),
      JSON.stringify(slice, null, 2),
      'utf-8',
    );
  }
}
