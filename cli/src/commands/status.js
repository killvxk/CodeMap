import path from 'path';
import fs from 'fs/promises';
import { loadGraph, loadMeta } from '../graph.js';

/**
 * Register the `status` command on the given Commander program.
 *
 * @param {import('commander').Command} program - The Commander program instance.
 */
export function registerStatusCommand(program) {
  program
    .command('status [dir]')
    .description('Show the status of the code graph for a project')
    .action(async (dir) => {
      const rootDir = path.resolve(dir || '.');
      const outputDir = path.join(rootDir, '.codemap');

      // Check if .codemap exists
      try {
        await fs.access(outputDir);
      } catch {
        console.log('No code graph found. Run "codegraph scan" first.');
        return;
      }

      let graph, meta;
      try {
        graph = await loadGraph(outputDir);
        meta = await loadMeta(outputDir);
      } catch (err) {
        console.error(`Error loading code graph: ${err.message}`);
        return;
      }

      // Print status
      console.log(`Project: ${graph.project.name}`);
      console.log(`Scanned at: ${graph.scannedAt}`);
      console.log(`Commit: ${graph.commitHash || '(none)'}`);
      console.log(`Files: ${graph.summary.totalFiles}`);
      console.log(`Functions: ${graph.summary.totalFunctions}`);
      console.log(`Classes: ${graph.summary.totalClasses}`);
      console.log(`Modules: ${graph.summary.modules.join(', ')}`);

      // Languages
      const langEntries = Object.entries(graph.summary.languages);
      if (langEntries.length > 0) {
        const langStr = langEntries.map(([lang, count]) => `${lang}(${count})`).join(', ');
        console.log(`Languages: ${langStr}`);
      }

      // Last update from meta
      if (meta.lastScanAt) {
        console.log(`Last update: ${meta.lastScanAt}`);
      }

      // Dirty files: files whose current hash differs from meta.fileHashes
      // (We just report what's in meta for now; actual dirty detection
      // would require re-traversing and re-hashing which is what `update` does)
      const fileHashCount = meta.fileHashes ? Object.keys(meta.fileHashes).length : 0;
      console.log(`Tracked files: ${fileHashCount}`);
    });
}
