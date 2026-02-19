import path from 'path';
import { loadGraph } from '../graph.js';
import { querySymbol, queryModule, queryDependants, queryDependencies } from '../query.js';

/**
 * Register the `query` command on the given Commander program.
 *
 * @param {import('commander').Command} program - The Commander program instance.
 */
export function registerQueryCommand(program) {
  program
    .command('query <symbol>')
    .description('Query the code graph for a symbol or module')
    .option('--type <type>', 'Filter by type: function, class, or type')
    .option('--dir <dir>', 'Project directory', '.')
    .action(async (symbol, options) => {
      const rootDir = path.resolve(options.dir);
      const outputDir = path.join(rootDir, '.codemap');

      let graph;
      try {
        graph = await loadGraph(outputDir);
      } catch {
        console.error('No code graph found. Run "codegraph scan" first.');
        process.exit(1);
      }

      // First try as module name
      const moduleResult = queryModule(graph, symbol);
      if (moduleResult) {
        console.log(`Module: ${moduleResult.name}`);
        console.log(`  Files: ${moduleResult.files.join(', ')}`);
        console.log(`  Depends on: ${moduleResult.dependsOn.length > 0 ? moduleResult.dependsOn.join(', ') : '(none)'}`);
        console.log(`  Depended by: ${moduleResult.dependedBy.length > 0 ? moduleResult.dependedBy.join(', ') : '(none)'}`);

        const deps = queryDependencies(graph, symbol);
        const dependants = queryDependants(graph, symbol);
        console.log(`  Dependencies: ${deps.length}`);
        console.log(`  Dependants: ${dependants.length}`);
        return;
      }

      // Then try as symbol name
      const symbolResults = querySymbol(graph, symbol, {
        type: options.type || undefined,
      });

      if (symbolResults.length === 0) {
        console.log(`No results found for "${symbol}".`);
        return;
      }

      console.log(`Found ${symbolResults.length} result(s) for "${symbol}":\n`);
      for (const result of symbolResults) {
        console.log(`  [${result.kind}] ${result.name}`);
        if (result.signature) {
          console.log(`    Signature: ${result.signature}`);
        }
        console.log(`    File: ${result.file}`);
        console.log(`    Module: ${result.module}`);
        console.log(`    Lines: ${result.lines.start}-${result.lines.end}`);
        if (result.calledBy.length > 0) {
          console.log(`    Called by: ${result.calledBy.join(', ')}`);
        }
      }
    });
}
