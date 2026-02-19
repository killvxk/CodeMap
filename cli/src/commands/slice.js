import path from 'path';
import { loadGraph } from '../graph.js';
import { generateOverview, buildModuleSlice, getModuleSliceWithDeps } from '../slicer.js';

/**
 * Register the `slice` command on the given Commander program.
 *
 * @param {import('commander').Command} program - The Commander program instance.
 */
export function registerSliceCommand(program) {
  program
    .command('slice [module]')
    .description('Output module slice or overview as JSON')
    .option('--with-deps', 'Include dependency info in module slice')
    .option('--dir <dir>', 'Project directory', '.')
    .action(async (moduleName, options) => {
      const rootDir = path.resolve(options.dir);
      const outputDir = path.join(rootDir, '.codemap');

      let graph;
      try {
        graph = await loadGraph(outputDir);
      } catch {
        console.error('No code graph found. Run "codegraph scan" first.');
        process.exit(1);
      }

      if (!moduleName) {
        // No module specified: output overview JSON
        const overview = generateOverview(graph);
        console.log(JSON.stringify(overview, null, 2));
        return;
      }

      // Module specified
      if (!graph.modules[moduleName]) {
        console.error(`Module "${moduleName}" not found in graph.`);
        console.error(`Available modules: ${Object.keys(graph.modules).join(', ')}`);
        process.exit(1);
      }

      if (options.withDeps) {
        const sliceWithDeps = getModuleSliceWithDeps(graph, moduleName);
        console.log(JSON.stringify(sliceWithDeps, null, 2));
      } else {
        // Build only the target module's slice instead of all modules
        const slice = buildModuleSlice(graph, moduleName, graph.modules[moduleName]);
        console.log(JSON.stringify(slice, null, 2));
      }
    });
}
