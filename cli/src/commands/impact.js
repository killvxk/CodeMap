import path from 'path';
import { loadGraph } from '../graph.js';
import { analyzeImpact } from '../impact.js';

/**
 * Register the `impact` command on the given Commander program.
 *
 * @param {import('commander').Command} program - The Commander program instance.
 */
export function registerImpactCommand(program) {
  program
    .command('impact <target>')
    .description('Analyze the impact of changes to a module or file')
    .option('--depth <depth>', 'Maximum BFS depth for transitive dependants', '3')
    .option('--dir <dir>', 'Project directory', '.')
    .action(async (target, options) => {
      const rootDir = path.resolve(options.dir);
      const outputDir = path.join(rootDir, '.codemap');

      let graph;
      try {
        graph = await loadGraph(outputDir);
      } catch {
        console.error('No code graph found. Run "codegraph scan" first.');
        process.exit(1);
      }

      const depth = parseInt(options.depth, 10) || 3;
      const result = analyzeImpact(graph, target, { depth });

      console.log(`Impact analysis for: ${target}`);
      console.log(`  Target type: ${result.targetType}`);
      console.log(`  Target module: ${result.targetModule}`);
      console.log(`  Direct dependants: ${result.directDependants.length > 0 ? result.directDependants.join(', ') : '(none)'}`);
      console.log(`  Transitive dependants: ${result.transitiveDependants.length > 0 ? result.transitiveDependants.join(', ') : '(none)'}`);
      console.log(`  Impacted modules (${result.impactedModules.length}): ${result.impactedModules.join(', ')}`);
      console.log(`  Impacted files (${result.impactedFiles.length}):`);
      for (const file of result.impactedFiles) {
        console.log(`    - ${file}`);
      }
    });
}
